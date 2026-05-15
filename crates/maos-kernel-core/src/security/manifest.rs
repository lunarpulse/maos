#![forbid(unsafe_code)]

//! Manifest section parsing.
//!
//! Story 1b.3 shipped `[sandbox]` and `[resources]` parsers; Story 1b.5c
//! extends to the remaining six sections present in
//! `spirits/hello-spirit/manifest.toml`: `[class]`,
//! `[capabilities.required]`, `[posture]`, `[output_shape]`, `[budget]`,
//! `[author]`. Each section carries `#[serde(deny_unknown_fields)]` so a
//! typo'd field becomes `ManifestError::Toml(…)` at parse time, not a
//! silent default-fill (Precondition 5 / Decision Register from 1b.3).
//!
//! The remaining architecture §5.1 sections (`forbidden_capabilities`,
//! `lifecycle`, `hot_swap`, etc.) are out of scope for v0.1-β and will
//! land at Epic 7 (Story 7.1 / 7.3).

use maos_domain::invariants::i9::SandboxTier;

/// Error raised during manifest section parsing or validation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ManifestError {
    #[error("sandbox tier parse failed: {0}")]
    TierParse(String),
    #[error("resource cap out of range: {field} = {value}")]
    CapOutOfRange { field: String, value: u32 },
    #[error("TOML deserialization failed: {0}")]
    Toml(String),
}

// Story 1b.5c — post-parse validation errors are surfaced via
// `ManifestError::Toml` with a stable `validation failed for {field}: {reason}`
// prefix so the public ABI of `ManifestError` remains exactly what Story 1b.3
// froze (`TierParse` + `CapOutOfRange` + `Toml`). Tests match the prefix or
// `field == "<section>.<field>"` substring; see [`validation_msg`].

/// Canonical formatter for validation errors carried as [`ManifestError::Toml`].
/// Keeps the message shape diff-stable; tests match the `validation failed for {field}:` prefix.
fn validation_msg(field: &str, reason: &str) -> String {
    format!("validation failed for {field}: {reason}")
}

/// The `[sandbox]` manifest section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SandboxConfig {
    pub tier: SandboxTier,
}

impl SandboxConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSandboxConfig = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        Ok(SandboxConfig { tier: raw.tier })
    }
}

/// The `[resources]` manifest section.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceCaps {
    pub cpu_max_pct: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub fd_max: Option<u32>,
}

impl ResourceCaps {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawResourceCaps = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

/// Resolved resource caps — strictest-of (manifest, operator-policy).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedCaps {
    pub cpu_max_pct: Option<u32>,
    pub memory_max_mb: Option<u32>,
    pub fd_max: Option<u32>,
}

/// Take the stricter (lower/None-patched) of two cap sources.
pub fn resolve_caps(manifest: &ResourceCaps, operator: &ResourceCaps) -> ResolvedCaps {
    ResolvedCaps {
        cpu_max_pct: strictest_opt(manifest.cpu_max_pct, operator.cpu_max_pct),
        memory_max_mb: strictest_opt(manifest.memory_max_mb, operator.memory_max_mb),
        fd_max: strictest_opt(manifest.fd_max, operator.fd_max),
    }
}

fn strictest_opt(a: Option<u32>, b: Option<u32>) -> Option<u32> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

// ------------------------------------------------------------------
// serde helpers
// ------------------------------------------------------------------

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSandboxConfig {
    #[serde(default = "default_tier")]
    tier: SandboxTier,
}

fn default_tier() -> SandboxTier {
    SandboxTier::DEFAULT_FLOOR
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawResourceCaps {
    cpu_max_pct: Option<u32>,
    memory_max_mb: Option<u32>,
    fd_max: Option<u32>,
}

impl RawResourceCaps {
    fn validate(self) -> Result<ResourceCaps, ManifestError> {
        let num_cpus = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);
        if let Some(v) = self.cpu_max_pct {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "cpu_max_pct".into(), value: v });
            }
            if v > 100 * num_cpus {
                return Err(ManifestError::CapOutOfRange { field: "cpu_max_pct".into(), value: v });
            }
        }
        if let Some(v) = self.memory_max_mb {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "memory_max_mb".into(), value: v });
            }
        }
        if let Some(v) = self.fd_max {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange { field: "fd_max".into(), value: v });
            }
        }
        Ok(ResourceCaps {
            cpu_max_pct: self.cpu_max_pct,
            memory_max_mb: self.memory_max_mb,
            fd_max: self.fd_max,
        })
    }
}

// ------------------------------------------------------------------
// [class] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// The `[class]` manifest section — Spirit identity + ABI + trust tier.
#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSection {
    pub name: String,
    pub version: String,
    pub abi: String,
    pub manifest_schema_version: u32,
    pub min_substrate_version: String,
    pub forms: Vec<String>,
    pub trust_tier: String,
    pub description: String,
}

impl ClassSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawClassSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClassSection {
    name: String,
    version: String,
    abi: String,
    manifest_schema_version: u32,
    min_substrate_version: String,
    forms: Vec<String>,
    trust_tier: String,
    description: String,
}

impl RawClassSection {
    fn validate(self) -> Result<ClassSection, ManifestError> {
        // name: non-empty; max 128 chars; [a-z0-9-] post-parse.
        if self.name.is_empty() {
            return Err(ManifestError::Toml(validation_msg("class.name", "empty")));
        }
        if self.name.len() > 128 {
            return Err(ManifestError::Toml(validation_msg(
                "class.name",
                &format!("len {} > 128", self.name.len()),
            )));
        }
        if !self.name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(ManifestError::Toml(validation_msg("class.name", "must be [a-z0-9-] only")));
        }
        // version: semver-ish (best-effort major.minor.patch).
        if self.version.split('.').count() < 3 {
            return Err(ManifestError::Toml(validation_msg("class.version", &format!("not semver: {}", self.version))));
        }
        // abi: format `<major>.<minor>`; v0.1-β only accepts `"1.0"`.
        if self.abi != "1.0" {
            return Err(ManifestError::Toml(validation_msg("class.abi", &format!("v0.1-β only accepts \"1.0\", got {}", self.abi))));
        }
        // manifest_schema_version: v0.1-β only accepts 1.
        if self.manifest_schema_version != 1 {
            return Err(ManifestError::Toml(validation_msg("class.manifest_schema_version", &format!("v0.1-β only accepts 1, got {}", self.manifest_schema_version))));
        }
        // min_substrate_version: best-effort semver-ish (allow pre-release suffix).
        let prefix = self
            .min_substrate_version
            .split('-')
            .next()
            .unwrap_or_default();
        if prefix.split('.').count() < 3 {
            return Err(ManifestError::Toml(validation_msg("class.min_substrate_version", &format!("not semver: {}", self.min_substrate_version))));
        }
        // forms: non-empty + every value ∈ {rust-inproc, subprocess}.
        if self.forms.is_empty() {
            return Err(ManifestError::Toml(validation_msg("class.forms", "must be non-empty")));
        }
        for f in &self.forms {
            if !matches!(f.as_str(), "rust-inproc" | "subprocess") {
                return Err(ManifestError::Toml(validation_msg("class.forms", &format!("unknown form: {f}"))));
            }
        }
        // trust_tier ∈ {local, org-internal, public-untrusted}.
        if !matches!(
            self.trust_tier.as_str(),
            "local" | "org-internal" | "public-untrusted"
        ) {
            return Err(ManifestError::Toml(validation_msg("class.trust_tier", &format!("unknown trust_tier: {}", self.trust_tier))));
        }
        // description: non-empty + ≤4 KiB.
        if self.description.is_empty() {
            return Err(ManifestError::Toml(validation_msg("class.description", "empty")));
        }
        if self.description.len() > 4096 {
            return Err(ManifestError::Toml(validation_msg(
                "class.description",
                &format!("len {} > 4096", self.description.len()),
            )));
        }

        Ok(ClassSection {
            name: self.name,
            version: self.version,
            abi: self.abi,
            manifest_schema_version: self.manifest_schema_version,
            min_substrate_version: self.min_substrate_version,
            forms: self.forms,
            trust_tier: self.trust_tier,
            description: self.description,
        })
    }
}

// ------------------------------------------------------------------
// [capabilities.required] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// The `[capabilities.required]` manifest section. v0.1-β only carries
/// `provider.complete` (the Inference Port provider call); the remaining
/// capability classes (`fs`, `net`, `iac`, etc.) land at Epic 7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitiesRequired {
    pub provider: ProviderCapabilities,
}

#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub complete: Vec<String>,
}

impl CapabilitiesRequired {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawCapabilitiesRequired =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilitiesRequired {
    provider: RawProviderCapabilities,
}

#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProviderCapabilities {
    complete: Vec<String>,
}

impl RawCapabilitiesRequired {
    fn validate(self) -> Result<CapabilitiesRequired, ManifestError> {
        if self.provider.complete.is_empty() {
            return Err(ManifestError::Toml(validation_msg("capabilities.required.provider.complete", "must be non-empty")));
        }
        for v in &self.provider.complete {
            if v.len() > 128 {
                return Err(ManifestError::Toml(validation_msg("capabilities.required.provider.complete", &format!("entry > 128 chars: {v}"))));
            }
        }
        Ok(CapabilitiesRequired {
            provider: ProviderCapabilities {
                complete: self.provider.complete,
            },
        })
    }
}

// ------------------------------------------------------------------
// [posture] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// Posture states per architecture §5.1 — the autonomy spectrum.
/// Enforcement (`allowed_max >= default`, posture-shift propagation,
/// intent_class binding) lands at Epic 3 (Story 3.x); v0.1-β only parses
/// + statically validates the ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Posture {
    Cautious,
    Assistive,
    AutonomousWithHalt,
    Autonomous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostureSection {
    pub default: Posture,
    pub allowed_max: Posture,
}

impl PostureSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawPostureSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPostureSection {
    default: Posture,
    allowed_max: Posture,
}

impl RawPostureSection {
    fn validate(self) -> Result<PostureSection, ManifestError> {
        if self.allowed_max < self.default {
            return Err(ManifestError::Toml(validation_msg(
                "posture.allowed_max",
                &format!(
                    "must be >= default ({:?} < {:?})",
                    self.allowed_max, self.default
                ),
            )));
        }
        Ok(PostureSection {
            default: self.default,
            allowed_max: self.allowed_max,
        })
    }
}

// ------------------------------------------------------------------
// [output_shape] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// The `[output_shape]` manifest section — declared required fields the
/// Spirit's response must carry (FR58 mandate; the orchestrator verifies
/// this shape at admission).
#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputShape {
    pub required_fields: Vec<String>,
}

impl OutputShape {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawOutputShape =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutputShape {
    required_fields: Vec<String>,
}

impl RawOutputShape {
    fn validate(self) -> Result<OutputShape, ManifestError> {
        if self.required_fields.is_empty() {
            return Err(ManifestError::Toml(validation_msg("output_shape.required_fields", "must be non-empty")));
        }
        if self.required_fields.len() > 32 {
            return Err(ManifestError::Toml(validation_msg(
                "output_shape.required_fields",
                &format!("len {} > 32", self.required_fields.len()),
            )));
        }
        Ok(OutputShape {
            required_fields: self.required_fields,
        })
    }
}

// ------------------------------------------------------------------
// [budget] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// The `[budget]` manifest section — declared inference budget. Enforcement
/// (rate limits + per-call gating against `time_cap_seconds`) lands at
/// Epic 5 / Epic 6; v0.1-β parses + statically validates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budget {
    pub context_window_size: u32,
    pub time_cap_seconds: u32,
}

impl Budget {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawBudget =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBudget {
    context_window_size: u32,
    time_cap_seconds: u32,
}

impl RawBudget {
    fn validate(self) -> Result<Budget, ManifestError> {
        if self.context_window_size == 0 {
            return Err(ManifestError::Toml(validation_msg("budget.context_window_size", "must be > 0")));
        }
        if self.context_window_size > (1u32 << 24) {
            return Err(ManifestError::Toml(validation_msg("budget.context_window_size", &format!("{} > 2^24", self.context_window_size))));
        }
        if self.time_cap_seconds == 0 {
            return Err(ManifestError::Toml(validation_msg("budget.time_cap_seconds", "must be > 0")));
        }
        if self.time_cap_seconds > 86_400 {
            return Err(ManifestError::Toml(validation_msg(
                "budget.time_cap_seconds",
                &format!("{} > 86400 (1 day)", self.time_cap_seconds),
            )));
        }
        Ok(Budget {
            context_window_size: self.context_window_size,
            time_cap_seconds: self.time_cap_seconds,
        })
    }
}

// ------------------------------------------------------------------
// [author] section (Story 1b.5c, AC3)
// ------------------------------------------------------------------

/// The `[author]` manifest section — Spirit provenance + optional
/// homepage URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub homepage: Option<String>,
}

impl Author {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawAuthor =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAuthor {
    name: String,
    homepage: Option<String>,
}

impl RawAuthor {
    fn validate(self) -> Result<Author, ManifestError> {
        if self.name.is_empty() {
            return Err(ManifestError::Toml(validation_msg("author.name", "empty")));
        }
        if let Some(ref h) = self.homepage {
            if !(h.starts_with("http://") || h.starts_with("https://")) {
                return Err(ManifestError::Toml(validation_msg("author.homepage", &format!("must start with http:// or https://, got: {h}"))));
            }
        }
        Ok(Author {
            name: self.name,
            homepage: self.homepage,
        })
    }
}

// ------------------------------------------------------------------
// Tests (NFR-Test-13 ≥3 cases per field)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- SandboxConfig ----

    #[test]
    fn sandbox_config_well_formed() {
        let cfg = SandboxConfig::from_toml_str(r#"tier = "T2""#).unwrap();
        assert_eq!(cfg.tier, SandboxTier::T2);
    }

    #[test]
    fn sandbox_config_malformed_rejected() {
        let err = SandboxConfig::from_toml_str(r#"tier = "t2""#).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    #[test]
    fn sandbox_config_edge_missing_uses_default() {
        let cfg = SandboxConfig::from_toml_str("").unwrap();
        assert_eq!(cfg.tier, SandboxTier::DEFAULT_FLOOR);
    }

    #[test]
    fn sandbox_config_t3_parseable_but_rejected_at_admission() {
        let cfg = SandboxConfig::from_toml_str(r#"tier = "T3""#).unwrap();
        assert_eq!(cfg.tier, SandboxTier::T3);
    }

    // ---- ResourceCaps ----

    #[test]
    fn resource_caps_well_formed() {
        let caps = ResourceCaps::from_toml_str("cpu_max_pct = 50\nmemory_max_mb = 512\nfd_max = 64").unwrap();
        assert_eq!(caps.cpu_max_pct, Some(50));
        assert_eq!(caps.memory_max_mb, Some(512));
        assert_eq!(caps.fd_max, Some(64));
    }

    #[test]
    fn resource_caps_malformed_zero_rejected() {
        let err = ResourceCaps::from_toml_str(r#"memory_max_mb = 0"#).unwrap_err();
        assert!(matches!(err, ManifestError::CapOutOfRange { field, .. } if field == "memory_max_mb"));
    }

    #[test]
    fn resource_caps_edge_missing_all_optional() {
        let caps = ResourceCaps::from_toml_str("").unwrap();
        assert_eq!(caps.cpu_max_pct, None);
        assert_eq!(caps.memory_max_mb, None);
        assert_eq!(caps.fd_max, None);
    }

    #[test]
    fn resource_caps_partial_present() {
        let caps = ResourceCaps::from_toml_str(r#"cpu_max_pct = 75"#).unwrap();
        assert_eq!(caps.cpu_max_pct, Some(75));
        assert_eq!(caps.memory_max_mb, None);
    }

    // ---- resolve_caps ----

    #[test]
    fn resolve_caps_manifest_wins_when_tighter() {
        let m = ResourceCaps { cpu_max_pct: Some(30), memory_max_mb: Some(256), fd_max: None };
        let o = ResourceCaps { cpu_max_pct: Some(50), memory_max_mb: Some(512), fd_max: None };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(30));
        assert_eq!(r.memory_max_mb, Some(256));
    }

    #[test]
    fn resolve_caps_operator_wins_when_tighter() {
        let m = ResourceCaps { cpu_max_pct: Some(80), memory_max_mb: None, fd_max: Some(128) };
        let o = ResourceCaps { cpu_max_pct: Some(40), memory_max_mb: Some(1024), fd_max: Some(64) };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(40));
        assert_eq!(r.memory_max_mb, Some(1024));
        assert_eq!(r.fd_max, Some(64));
    }

    #[test]
    fn resolve_caps_none_falls_through() {
        let m = ResourceCaps { cpu_max_pct: None, memory_max_mb: Some(256), fd_max: None };
        let o = ResourceCaps { cpu_max_pct: Some(50), memory_max_mb: None, fd_max: Some(32) };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(50));
        assert_eq!(r.memory_max_mb, Some(256));
        assert_eq!(r.fd_max, Some(32));
    }

    #[test]
    fn resolve_caps_both_none_none() {
        let m = ResourceCaps::default();
        let o = ResourceCaps::default();
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, None);
        assert_eq!(r.memory_max_mb, None);
        assert_eq!(r.fd_max, None);
    }

    // ---- ClassSection (Story 1b.5c, AC3) ----

    fn class_toml_full() -> String {
        r#"
name = "hello-spirit"
version = "0.1.0"
abi = "1.0"
manifest_schema_version = 1
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "local"
description = "MAOS reference Spirit"
"#
        .into()
    }

    #[test]
    fn class_section_well_formed() {
        let c = ClassSection::from_toml_str(&class_toml_full()).unwrap();
        assert_eq!(c.name, "hello-spirit");
        assert_eq!(c.abi, "1.0");
        assert_eq!(c.trust_tier, "local");
        assert_eq!(c.forms, vec!["rust-inproc"]);
    }

    #[test]
    fn class_section_rejects_empty_name() {
        let s = class_toml_full().replace(r#""hello-spirit""#, r#""""#);
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.name")));
    }

    #[test]
    fn class_section_rejects_non_semver_version() {
        let s = class_toml_full().replace(r#"version = "0.1.0""#, r#"version = "0.1""#);
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.version")));
    }

    #[test]
    fn class_section_rejects_bad_abi() {
        let s = class_toml_full().replace(r#"abi = "1.0""#, r#"abi = "2.0""#);
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.abi")));
    }

    #[test]
    fn class_section_rejects_zero_schema_version() {
        let s = class_toml_full()
            .replace("manifest_schema_version = 1", "manifest_schema_version = 0");
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.manifest_schema_version")));
    }

    #[test]
    fn class_section_rejects_unknown_form() {
        let s = class_toml_full().replace(r#"["rust-inproc"]"#, r#"["wasm"]"#);
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.forms")));
    }

    #[test]
    fn class_section_rejects_unknown_trust_tier() {
        let s = class_toml_full().replace(r#"trust_tier = "local""#, r#"trust_tier = "premium""#);
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.trust_tier")));
    }

    #[test]
    fn class_section_rejects_typo_field() {
        // deny_unknown_fields discipline — a typo'd field is a TOML error.
        let s = class_toml_full().replace("trust_tier =", "trust_teir =");
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    // ---- CapabilitiesRequired ----

    #[test]
    fn capabilities_required_well_formed() {
        let s = r#"provider.complete = ["anthropic.claude-3-haiku-20240307"]"#;
        let c = CapabilitiesRequired::from_toml_str(s).unwrap();
        assert_eq!(c.provider.complete.len(), 1);
    }

    #[test]
    fn capabilities_required_rejects_empty_complete() {
        let s = r#"provider.complete = []"#;
        let err = CapabilitiesRequired::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("capabilities.required.provider.complete")));
    }

    #[test]
    fn capabilities_required_rejects_overlong_entry() {
        let long = "a".repeat(200);
        let s = format!(r#"provider.complete = ["{long}"]"#);
        let err = CapabilitiesRequired::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("capabilities.required.provider.complete")));
    }

    #[test]
    fn capabilities_required_rejects_unknown_field() {
        let s = r#"provider.complete = ["x"]
extra = "y""#;
        let err = CapabilitiesRequired::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    // ---- PostureSection ----

    #[test]
    fn posture_section_well_formed() {
        let s = r#"default = "assistive"
allowed_max = "assistive""#;
        let p = PostureSection::from_toml_str(s).unwrap();
        assert_eq!(p.default, Posture::Assistive);
        assert_eq!(p.allowed_max, Posture::Assistive);
    }

    #[test]
    fn posture_section_rejects_unknown_variant() {
        let s = r#"default = "feral"
allowed_max = "assistive""#;
        let err = PostureSection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(_)));
    }

    #[test]
    fn posture_section_rejects_max_below_default() {
        let s = r#"default = "autonomous"
allowed_max = "cautious""#;
        let err = PostureSection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("posture.allowed_max")));
    }

    #[test]
    fn posture_section_edge_autonomous_with_halt_lt_autonomous() {
        let s = r#"default = "autonomous-with-halt"
allowed_max = "autonomous""#;
        let p = PostureSection::from_toml_str(s).unwrap();
        assert_eq!(p.default, Posture::AutonomousWithHalt);
        assert_eq!(p.allowed_max, Posture::Autonomous);
    }

    // ---- OutputShape ----

    #[test]
    fn output_shape_well_formed() {
        let s = r#"required_fields = ["introduction", "capability_scope"]"#;
        let o = OutputShape::from_toml_str(s).unwrap();
        assert_eq!(o.required_fields.len(), 2);
    }

    #[test]
    fn output_shape_rejects_empty() {
        let s = r#"required_fields = []"#;
        let err = OutputShape::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("output_shape.required_fields")));
    }

    #[test]
    fn output_shape_rejects_overlong_list() {
        let v: Vec<String> = (0..33).map(|i| format!("f{i}")).collect();
        let joined = v
            .iter()
            .map(|f| format!(r#""{f}""#))
            .collect::<Vec<_>>()
            .join(",");
        let s = format!(r#"required_fields = [{joined}]"#);
        let err = OutputShape::from_toml_str(&s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("validation failed for")));
    }

    #[test]
    fn output_shape_edge_single_field() {
        let s = r#"required_fields = ["x"]"#;
        let o = OutputShape::from_toml_str(s).unwrap();
        assert_eq!(o.required_fields, vec!["x"]);
    }

    // ---- Budget ----

    #[test]
    fn budget_well_formed() {
        let s = "context_window_size = 4096\ntime_cap_seconds = 30";
        let b = Budget::from_toml_str(s).unwrap();
        assert_eq!(b.context_window_size, 4096);
        assert_eq!(b.time_cap_seconds, 30);
    }

    #[test]
    fn budget_rejects_zero_context_window_size() {
        let s = "context_window_size = 0\ntime_cap_seconds = 30";
        let err = Budget::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.context_window_size")));
    }

    #[test]
    fn budget_rejects_zero_time_cap_seconds() {
        let s = "context_window_size = 4096\ntime_cap_seconds = 0";
        let err = Budget::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.time_cap_seconds")));
    }

    #[test]
    fn budget_rejects_excessive_time_cap() {
        let s = "context_window_size = 4096\ntime_cap_seconds = 86401";
        let err = Budget::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.time_cap_seconds")));
    }

    // ---- Author ----

    #[test]
    fn author_well_formed_with_homepage() {
        let s = r#"name = "MAOS"
homepage = "https://example.org""#;
        let a = Author::from_toml_str(s).unwrap();
        assert_eq!(a.name, "MAOS");
        assert_eq!(a.homepage.as_deref(), Some("https://example.org"));
    }

    #[test]
    fn author_well_formed_without_homepage() {
        let s = r#"name = "MAOS""#;
        let a = Author::from_toml_str(s).unwrap();
        assert_eq!(a.homepage, None);
    }

    #[test]
    fn author_rejects_empty_name() {
        let s = r#"name = """#;
        let err = Author::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("author.name")));
    }

    #[test]
    fn author_rejects_bad_homepage_scheme() {
        let s = r#"name = "MAOS"
homepage = "ftp://example.org""#;
        let err = Author::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("author.homepage")));
    }
}
