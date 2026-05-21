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

/// Convert a `CapabilitiesRequired` manifest declaration to a `Vec<Scope>`.
///
/// v0.1-β only produces `Scope::ProviderInfer` — the provider prefix is
/// extracted before the first `.` in each entry (e.g., `"anthropic.claude-3-haiku-20240307"`
/// → `ProviderInfer { provider: "anthropic" }`). Other scope classes
/// (fs, net, iac, mem) return a clearly-commented TODO and are not
/// produced — they ship in Epic 7.
pub fn capabilities_required_to_scopes(caps: &CapabilitiesRequired) -> Vec<maos_domain::invariants::i1::Scope> {
    caps.provider
        .complete
        .iter()
        .map(|entry| {
            let provider = entry.split('.').next().unwrap_or(entry);
            if provider.is_empty() {
                return maos_domain::invariants::i1::Scope::ProviderInfer {
                    provider: entry.clone(),
                };
            }
            maos_domain::invariants::i1::Scope::ProviderInfer {
                provider: provider.to_string(),
            }
        })
        .collect()
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
    /// Story 3.2: not a runtime shift target; use cautious /
    /// assistive / autonomous-with-halt via `maosctl posture --shift`.
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
        // Story 2.1 (AC3): reject field names containing whitespace.
        for name in &self.required_fields {
            if name.contains(|c: char| c.is_whitespace()) {
                return Err(ManifestError::Toml(validation_msg(
                    "output_shape.required_fields",
                    &format!("whitespace in field name '{}'", name),
                )));
            }
        }
        // Story 2.1 (AC3): reject duplicate field names.
        {
            let mut seen = std::collections::HashSet::new();
            for name in &self.required_fields {
                if !seen.insert(name.as_str()) {
                    return Err(ManifestError::Toml(validation_msg(
                        "output_shape.required_fields",
                        &format!("duplicate field name '{}'", name),
                    )));
                }
            }
        }
        Ok(OutputShape {
            required_fields: self.required_fields,
        })
    }
}

// ------------------------------------------------------------------
// OutputShapePredicate — Story 2.1 (AC3)
// ------------------------------------------------------------------

/// Structural predicate built from `[output_shape] required_fields`.
///
/// Scaffolding for Story 7.3 fail-loud enforcement: the predicate is
/// constructed at admission and held by the kernel; the emit-side
/// enforcement (`check()` on every frame emit) lands at E7.
#[maos_attrs::i9_exempt(reason = "manifest-derived predicate; constructed at admission and held in SandboxSpec — dropped after spawn, no kernel persistence")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputShapePredicate {
    fields: Vec<String>,
}

impl OutputShapePredicate {
    /// Construct the predicate from an already-validated `OutputShape`.
    pub fn from(output_shape: &OutputShape) -> Self {
        Self {
            fields: output_shape.required_fields.clone(),
        }
    }

    /// Check whether `value` satisfies the predicate.
    ///
    /// Returns `Ok(())` iff every `required_fields` entry is present
    /// as a top-level key with a non-null value. Errors on the first
    /// missing or null field in declaration order.
    pub fn check(&self, value: &serde_json::Value) -> Result<(), OutputShapeViolation> {
        for field in &self.fields {
            match value.get(field) {
                None => {
                    return Err(OutputShapeViolation::MissingField {
                        name: field.clone(),
                    });
                }
                Some(serde_json::Value::Null) => {
                    return Err(OutputShapeViolation::NullField {
                        name: field.clone(),
                    });
                }
                Some(_) => {} // present and non-null — ok
            }
        }
        Ok(())
    }
}

/// Violation of the `[output_shape]` predicate at frame-emit time.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OutputShapeViolation {
    #[error("missing required field '{name}'")]
    MissingField { name: String },
    #[error("required field '{name}' is null")]
    NullField { name: String },
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
// [epistemic_policy] section (Story 3.2, AC1)
// ------------------------------------------------------------------

/// Halt action for a tag in `[epistemic_policy]` per architecture §4.6.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpistemicAction {
    VerbalizeOnly,
    Flag,
    Halt,
}

/// A single per-tag rule in `[epistemic_policy]` per architecture §4.6.1.
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicPolicyRule {
    pub tag: String,
    pub action: EpistemicAction,
    pub on_confidence_below: Option<f32>,
    pub on_evidence_conflict: Option<bool>,
    /// Story 4.2 — universal-arithmetic predicate. When only
    /// `on_confidence_below` is set, this field is auto-desugared to
    /// `Some(ScalarPredicate::Below { threshold })` during `validate()`.
    /// `on_evidence_conflict` remains a non-scalar boolean guard.
    pub predicate: Option<ScalarPredicate>,
}

impl EpistemicPolicyRule {
    /// Construct a rule with `predicate: None` — for manual constructions
    /// (e.g., in tests). The `predicate` field is auto-filled from the
    /// four `on_value_*` TOML fields during `from_toml_str` parsing.
    pub fn new(
        tag: String,
        action: EpistemicAction,
        on_confidence_below: Option<f32>,
        on_evidence_conflict: Option<bool>,
        predicate: Option<ScalarPredicate>,
    ) -> Self {
        Self {
            tag,
            action,
            on_confidence_below,
            on_evidence_conflict,
            predicate,
        }
    }
}

/// Story 4.2 — one of the four universal-arithmetic ADR-022 predicates.
///
/// The kernel dispatches to `CapabilityRegistryPort::on_value_above` /
/// `on_value_below` / `on_value_within` / `on_value_outside` based on
/// this variant. Constructed during manifest `validate()` from the
/// `on_value_*` TOML fields.
#[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarPredicate {
    /// `on_value_above = { threshold = 0.8 }` — fires when `value > threshold`.
    Above {
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        threshold: f32,
    },
    /// `on_value_below = { threshold = 0.2 }` — fires when `value < threshold`.
    Below {
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        threshold: f32,
    },
    /// `on_value_within = { lower = 0.4, upper = 0.6 }` — fires when `lower <= value <= upper`.
    Within {
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        lower: f32,
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        upper: f32,
    },
    /// `on_value_outside = { lower = 0.3, upper = 0.7 }` — fires when `value < lower || value > upper`.
    Outside {
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        lower: f32,
        #[doc = "Construct via [`ScalarPredicate`] derive(Deserialize); struct literals bypass NaN / range checks."]
        upper: f32,
    },
}

/// The `[epistemic_policy]` manifest section — per-tag halt policy rules
/// plus a default action that defaults to `VerbalizeOnly` (fail-open).
#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicPolicySection {
    pub rules: Vec<EpistemicPolicyRule>,
    pub default_action: EpistemicAction,
}

impl EpistemicPolicySection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawEpistemicPolicySection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }

    pub fn default_open_fail() -> Self {
        Self {
            rules: vec![],
            default_action: EpistemicAction::VerbalizeOnly,
        }
    }
}

#[maos_attrs::i9_exempt(reason = "manifest data; parsed-then-dropped at admission, no kernel persistence")]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEpistemicPolicySection {
    #[serde(default)]
    rules: Vec<RawEpistemicPolicyRule>,
    #[serde(default = "default_epistemic_action")]
    default_action: EpistemicAction,
}

fn default_epistemic_action() -> EpistemicAction {
    EpistemicAction::VerbalizeOnly
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEpistemicPolicyRule {
    tag: String,
    action: EpistemicAction,
    on_confidence_below: Option<f32>,
    on_evidence_conflict: Option<bool>,
    /// Story 4.2 — flattened optional predicate forms.
    on_value_above: Option<RawScalarPredicate>,
    on_value_below: Option<RawScalarPredicate>,
    on_value_within: Option<RawScalarPredicateWithin>,
    on_value_outside: Option<RawScalarPredicateWithin>,
}

/// Raw deserialization target for single-threshold predicate forms
/// (`on_value_above`, `on_value_below`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScalarPredicate {
    threshold: f32,
}

/// Raw deserialization target for two-bound predicate forms
/// (`on_value_within`, `on_value_outside`).
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScalarPredicateWithin {
    lower: f32,
    upper: f32,
}

impl RawEpistemicPolicySection {
    fn validate(self) -> Result<EpistemicPolicySection, ManifestError> {
        let rules: Vec<EpistemicPolicyRule> = self
            .rules
            .into_iter()
            .map(|r| {
                if r.tag.is_empty() {
                    return Err(ManifestError::Toml(validation_msg(
                        "epistemic_policy.rules",
                        "tag must be non-empty",
                    )));
                }
                if r.tag.contains(|c: char| c.is_whitespace()) {
                    return Err(ManifestError::Toml(validation_msg(
                        "epistemic_policy.rules",
                        &format!("whitespace in tag '{}'", r.tag),
                    )));
                }

                // Story 4.2 — collapse the four optional on_value_* fields
                // into a single Option<ScalarPredicate>.
                let predicate = collapse_predicate_fields(
                    &r.tag,
                    r.on_value_above,
                    r.on_value_below,
                    r.on_value_within,
                    r.on_value_outside,
                )?;

                // Reject rules carrying BOTH on_confidence_below and any
                // of the four predicate forms.
                if r.on_confidence_below.is_some() && predicate.is_some() {
                    return Err(ManifestError::Toml(validation_msg(
                        "epistemic_policy.rules",
                        &format!(
                            "rule '{}' carries both on_confidence_below and on_value_* — choose one",
                            r.tag
                        ),
                    )));
                }

                let on_confidence_below = r.on_confidence_below;
                if let Some(v) = r.on_confidence_below {
                    if !(0.0..=1.0).contains(&v) {
                        return Err(ManifestError::Toml(validation_msg(
                            "epistemic_policy.on_confidence_below",
                            "must be in [0.0, 1.0]",
                        )));
                    }
                }

                // When only on_confidence_below is set, auto-desugar to
                // predicate: Some(ScalarPredicate::Below { threshold })
                // (Story 3.2 backward compatibility).
                let predicate = if predicate.is_some() {
                    predicate
                } else if let Some(t) = r.on_confidence_below {
                    Some(ScalarPredicate::Below { threshold: t })
                } else {
                    None
                };

                Ok(EpistemicPolicyRule {
                    tag: r.tag,
                    action: r.action,
                    on_confidence_below: r.on_confidence_below,
                    on_evidence_conflict: r.on_evidence_conflict,
                    predicate,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut seen = std::collections::HashSet::new();
        for rule in &rules {
            if !seen.insert(&rule.tag) {
                return Err(ManifestError::Toml(validation_msg(
                    "epistemic_policy.rules",
                    &format!("duplicate tag '{}'", rule.tag),
                )));
            }
        }

        Ok(EpistemicPolicySection {
            rules,
            default_action: self.default_action,
        })
    }
}

/// Story 4.2 — collapse the four optional `on_value_*` raw fields into
/// exactly-one-or-none `Option<ScalarPredicate>`.
fn collapse_predicate_fields(
    tag: &str,
    above: Option<RawScalarPredicate>,
    below: Option<RawScalarPredicate>,
    within: Option<RawScalarPredicateWithin>,
    outside: Option<RawScalarPredicateWithin>,
) -> Result<Option<ScalarPredicate>, ManifestError> {
    let count = above.is_some() as u8
        + below.is_some() as u8
        + within.is_some() as u8
        + outside.is_some() as u8;
    if count > 1 {
        return Err(ManifestError::Toml(validation_msg(
            "epistemic_policy.rules",
            &format!(
                "rule '{}' carries multiple predicate forms — choose exactly one",
                tag
            ),
        )));
    }
    if let Some(raw) = above {
        if raw.threshold.is_nan() {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_above.threshold",
                "threshold must not be NaN",
            )));
        }
        return Ok(Some(ScalarPredicate::Above {
            threshold: raw.threshold,
        }));
    }
    if let Some(raw) = below {
        if raw.threshold.is_nan() {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_below.threshold",
                "threshold must not be NaN",
            )));
        }
        return Ok(Some(ScalarPredicate::Below {
            threshold: raw.threshold,
        }));
    }
    if let Some(raw) = within {
        if raw.lower.is_nan() || raw.upper.is_nan() {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_within",
                "lower/upper must not be NaN",
            )));
        }
        if raw.lower > raw.upper {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_within",
                &format!(
                    "lower ({}) > upper ({})",
                    raw.lower, raw.upper
                ),
            )));
        }
        return Ok(Some(ScalarPredicate::Within {
            lower: raw.lower,
            upper: raw.upper,
        }));
    }
    if let Some(raw) = outside {
        if raw.lower.is_nan() || raw.upper.is_nan() {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_outside",
                "lower/upper must not be NaN",
            )));
        }
        if raw.lower > raw.upper {
            return Err(ManifestError::Toml(validation_msg(
                "epistemic_policy.on_value_outside",
                &format!(
                    "lower ({}) > upper ({})",
                    raw.lower, raw.upper
                ),
            )));
        }
        return Ok(Some(ScalarPredicate::Outside {
            lower: raw.lower,
            upper: raw.upper,
        }));
    }
    Ok(None)
}

// ------------------------------------------------------------------
// [scheduling] section (Story 5.1, AC3)
// ------------------------------------------------------------------

/// The `[scheduling]` manifest section — priority-weighted cooperative
/// scheduling parameters. All fields carry `#[serde(default)]` so
/// manifests omitting the section inherit the defaults below.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulingSection {
    /// Priority weight for DRR dispatch [1, 255]; default 100.
    pub priority_weight: u8,
    /// Poll yield cap for cooperative dispatch; [1, 4096]; default 64.
    pub yield_every_polls: u32,
    /// Mailbox quiescence threshold for on_idle trigger; [100, 3600000] ms; default 30000.
    pub idle_window_ms: u32,
}

impl Default for SchedulingSection {
    fn default() -> Self {
        Self {
            priority_weight: 100,
            yield_every_polls: 64,
            idle_window_ms: 30000,
        }
    }
}

impl SchedulingSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSchedulingSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSchedulingSection {
    #[serde(default = "default_priority_weight")]
    priority_weight: u8,
    #[serde(default = "default_yield_every_polls")]
    yield_every_polls: u32,
    #[serde(default = "default_idle_window_ms")]
    idle_window_ms: u32,
}

fn default_priority_weight() -> u8 {
    100
}
fn default_yield_every_polls() -> u32 {
    64
}
fn default_idle_window_ms() -> u32 {
    30000
}

impl RawSchedulingSection {
    fn validate(self) -> Result<SchedulingSection, ManifestError> {
        if self.priority_weight < 1 || self.priority_weight > 255 {
            return Err(ManifestError::Toml(validation_msg(
                "scheduling.priority_weight",
                &format!(
                    "must be in [1, 255], got {}",
                    self.priority_weight
                ),
            )));
        }
        if self.yield_every_polls < 1 || self.yield_every_polls > 4096 {
            return Err(ManifestError::Toml(validation_msg(
                "scheduling.yield_every_polls",
                &format!(
                    "must be in [1, 4096], got {}",
                    self.yield_every_polls
                ),
            )));
        }
        if self.idle_window_ms < 100 || self.idle_window_ms > 3_600_000 {
            return Err(ManifestError::Toml(validation_msg(
                "scheduling.idle_window_ms",
                &format!(
                    "must be in [100, 3600000], got {}",
                    self.idle_window_ms
                ),
            )));
        }
        Ok(SchedulingSection {
            priority_weight: self.priority_weight,
            yield_every_polls: self.yield_every_polls,
            idle_window_ms: self.idle_window_ms,
        })
    }
}

// ------------------------------------------------------------------
// [lifecycle] section (Story 5.1, AC2)
// ------------------------------------------------------------------

/// Valid lifecycle hook names per the Spirit ABI (Story 2.1).
const VALID_HOOK_NAMES: &[&str] = &[
    "on_load",
    "on_start",
    "on_frame",
    "on_idle",
    "on_telemetry_event",
    "on_schedule",
    "on_swap_in",
    "on_pause",
    "on_resume",
    "on_unload",
    "on_consolidate",
];

/// The `[lifecycle]` manifest section — declared hook subset.
///
/// Empty `enabled_hooks` means "all hooks allowed" (matches
/// `kernel_invocation_allowed(&[], _) → true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleSection {
    pub enabled_hooks: Vec<String>,
}

impl Default for LifecycleSection {
    fn default() -> Self {
        Self {
            enabled_hooks: vec![],
        }
    }
}

impl LifecycleSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawLifecycleSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawLifecycleSection {
    #[serde(default)]
    enabled_hooks: Vec<String>,
}

impl RawLifecycleSection {
    fn validate(self) -> Result<LifecycleSection, ManifestError> {
        // Check all hook names are valid
        for hook in &self.enabled_hooks {
            if !VALID_HOOK_NAMES.contains(&hook.as_str()) {
                return Err(ManifestError::Toml(validation_msg(
                    "lifecycle.enabled_hooks",
                    &format!(
                        "unknown hook name '{}'; valid hooks: {}",
                        hook,
                        VALID_HOOK_NAMES.join(", ")
                    ),
                )));
            }
        }
        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        for hook in &self.enabled_hooks {
            if !seen.insert(hook) {
                return Err(ManifestError::Toml(validation_msg(
                    "lifecycle.enabled_hooks",
                    &format!("duplicate hook name '{}'", hook),
                )));
            }
        }
        Ok(LifecycleSection {
            enabled_hooks: self.enabled_hooks,
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

    // ---- OutputShape whitespace + duplicate Story 2.1 (AC3) ----

    #[test]
    fn output_shape_rejects_whitespace_in_field_name() {
        let s = r#"required_fields = ["with space"]"#;
        let err = OutputShape::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("whitespace in field name")),
            "expected whitespace rejection, got: {err}"
        );
    }

    #[test]
    fn output_shape_rejects_duplicate_field_name() {
        let s = r#"required_fields = ["a", "b", "a"]"#;
        let err = OutputShape::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("duplicate field name")),
            "expected duplicate rejection, got: {err}"
        );
    }

    // ---- OutputShapePredicate Story 2.1 (AC3) ----

    use serde_json::json;

    #[test]
    fn predicate_hits_on_hello_spirit_four_fields() {
        let shape = OutputShape {
            required_fields: vec![
                "introduction".into(),
                "capability_scope".into(),
                "halt_tags".into(),
                "transparency_log".into(),
            ],
        };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({
            "introduction": "Hello",
            "capability_scope": "provider:anthropic",
            "halt_tags": ["a"],
            "transparency_log": "hash",
            "extra": "ignored",
        });
        assert!(pred.check(&value).is_ok());
    }

    #[test]
    fn predicate_misses_first_field() {
        let shape = OutputShape { required_fields: vec!["a".into(), "b".into()] };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"b": 1});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::MissingField { name } if name == "a"));
    }

    #[test]
    fn predicate_misses_second_field() {
        let shape = OutputShape { required_fields: vec!["a".into(), "b".into()] };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"a": 1});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::MissingField { name } if name == "b"));
    }

    #[test]
    fn predicate_rejects_null_field() {
        let shape = OutputShape { required_fields: vec!["a".into()] };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"a": null});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::NullField { name } if name == "a"));
    }

    #[test]
    fn predicate_accepts_empty_object_for_empty_required_fields() {
        // Empty predicate trivially satisfied.
        let shape = OutputShape { required_fields: vec![] };
        let pred = OutputShapePredicate::from(&shape);
        assert!(pred.check(&json!({})).is_ok());
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

    // ---- EpistemicPolicySection (Story 3.2, AC1) ----

    #[test]
    fn epistemic_policy_well_formed_parses() {
        let s = r#"default_action = "verbalize_only"

[[rules]]
tag = "claim.security_vulnerability"
action = "halt"
on_confidence_below = 0.85
on_evidence_conflict = true"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.rules.len(), 1);
        assert_eq!(p.rules[0].tag, "claim.security_vulnerability");
        assert_eq!(p.rules[0].action, EpistemicAction::Halt);
        assert_eq!(p.rules[0].on_confidence_below, Some(0.85));
        assert_eq!(p.rules[0].on_evidence_conflict, Some(true));
        assert_eq!(p.default_action, EpistemicAction::VerbalizeOnly);
    }

    #[test]
    fn epistemic_policy_rejects_threshold_above_one() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = 1.5"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("epistemic_policy.on_confidence_below")));
    }

    #[test]
    fn epistemic_policy_rejects_threshold_below_zero() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = -0.1"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("epistemic_policy.on_confidence_below")));
    }

    #[test]
    fn epistemic_policy_rejects_duplicate_tag() {
        let s = r#"[[rules]]
tag = "a"
action = "verbalize_only"
[[rules]]
tag = "a"
action = "flag""#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("duplicate tag")));
    }

    #[test]
    fn epistemic_policy_rejects_empty_tag() {
        let s = r#"[[rules]]
tag = ""
action = "halt""#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("tag must be non-empty")));
    }

    #[test]
    fn epistemic_policy_default_action_defaults_to_verbalize_only_when_omitted() {
        let s = r#"[[rules]]
tag = "x"
action = "flag""#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.default_action, EpistemicAction::VerbalizeOnly);
    }

    #[test]
    fn epistemic_policy_rejects_whitespace_in_tag() {
        let s = r#"[[rules]]
tag = "with space"
action = "halt""#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("whitespace in tag")));
    }

    #[test]
    fn epistemic_policy_edge_case_multi_rule_no_qualifiers() {
        let s = r#"[[rules]]
tag = "a"
action = "verbalize_only"
[[rules]]
tag = "b"
action = "flag"
on_confidence_below = 0.0
[[rules]]
tag = "c"
action = "halt"
on_confidence_below = 1.0"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.rules.len(), 3);
        assert_eq!(p.rules[1].on_confidence_below, Some(0.0));
        assert_eq!(p.rules[2].on_confidence_below, Some(1.0));
    }

    #[test]
    fn epistemic_policy_malformed_fixture_rules_is_rejected() {
        let s = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/manifest/epistemic_policy/malformed-rejected/rules.toml"),
        )
        .unwrap();
        let err = EpistemicPolicySection::from_toml_str(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(_)),
            "malformed fixture must fail parse: {err}"
        );
    }

    #[test]
    fn epistemic_policy_malformed_fixture_default_action_is_rejected() {
        let s = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/manifest/epistemic_policy/malformed-rejected/default_action.toml"),
        )
        .unwrap();
        let err = EpistemicPolicySection::from_toml_str(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(_)),
            "malformed fixture must fail parse: {err}"
        );
    }

    // ---- Story 4.2 — ScalarPredicate form tests (Task 1.4) ----

    #[test]
    fn predicate_on_value_above_well_formed_parses() {
        let s = r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_above = { threshold = 0.8 }"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.rules.len(), 1);
        assert!(matches!(
            p.rules[0].predicate,
            Some(ScalarPredicate::Above { threshold }) if (threshold - 0.8).abs() < 0.001
        ));
    }

    #[test]
    fn predicate_on_value_below_well_formed_parses() {
        let s = r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_below = { threshold = 0.2 }"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert!(matches!(
            p.rules[0].predicate,
            Some(ScalarPredicate::Below { threshold }) if (threshold - 0.2).abs() < 0.001
        ));
    }

    #[test]
    fn predicate_on_value_within_well_formed_parses() {
        let s = r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_within = { lower = 0.4, upper = 0.6 }"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert!(matches!(
            p.rules[0].predicate,
            Some(ScalarPredicate::Within { lower, upper })
                if (lower - 0.4).abs() < 0.001 && (upper - 0.6).abs() < 0.001
        ));
    }

    #[test]
    fn predicate_on_value_outside_well_formed_parses() {
        let s = r#"[[rules]]
tag = "uncertainty"
action = "halt"
on_value_outside = { lower = 0.3, upper = 0.7 }"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert!(matches!(
            p.rules[0].predicate,
            Some(ScalarPredicate::Outside { lower, upper })
                if (lower - 0.3).abs() < 0.001 && (upper - 0.7).abs() < 0.001
        ));
    }

    #[test]
    fn predicate_rejects_nan_threshold_in_above() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_above = { threshold = nan }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("NaN")));
    }

    #[test]
    fn predicate_rejects_inverted_bounds_within() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_within = { lower = 0.7, upper = 0.3 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("lower") && msg.contains("upper")));
    }

    #[test]
    fn predicate_rejects_nan_threshold_in_below() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_below = { threshold = nan }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("NaN")));
    }

    #[test]
    fn predicate_rejects_nan_threshold_in_outside() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_outside = { lower = nan, upper = 0.5 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("NaN")));
    }

    #[test]
    fn predicate_rejects_inverted_bounds_outside() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_outside = { lower = 0.7, upper = 0.3 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("lower") && msg.contains("upper")));
    }

    #[test]
    fn predicate_rejects_both_forms_set() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_above = { threshold = 0.5 }
on_value_below = { threshold = 0.3 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("multiple predicate forms")));
    }

    #[test]
    fn predicate_rejects_both_confidence_below_and_on_value() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = 0.5
on_value_above = { threshold = 0.8 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(matches!(err, ManifestError::Toml(ref msg) if msg.contains("both on_confidence_below and on_value_*")));
    }

    #[test]
    fn predicate_on_confidence_below_desugars_to_below_predicate() {
        // Story 3.2 backward compat — on_confidence_below alone desugars to
        // predicate: Some(ScalarPredicate::Below { threshold })
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = 0.75"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.rules[0].on_confidence_below, Some(0.75));
        assert!(matches!(
            p.rules[0].predicate,
            Some(ScalarPredicate::Below { threshold }) if (threshold - 0.75).abs() < 0.001
        ));
    }

    #[test]
    fn predicate_3_2_shape_backward_compat_unchanged() {
        // Pre-4.2 manifest (only on_confidence_below + on_evidence_conflict)
        // MUST deserialize unchanged.
        let s = r#"default_action = "verbalize_only"

[[rules]]
tag = "claim.security_vulnerability"
action = "halt"
on_confidence_below = 0.85
on_evidence_conflict = true"#;
        let p = EpistemicPolicySection::from_toml_str(s).unwrap();
        assert_eq!(p.rules.len(), 1);
        assert_eq!(p.rules[0].tag, "claim.security_vulnerability");
        assert_eq!(p.rules[0].action, EpistemicAction::Halt);
        assert_eq!(p.rules[0].on_confidence_below, Some(0.85));
        assert_eq!(p.rules[0].on_evidence_conflict, Some(true));
        assert!(p.rules[0].predicate.is_some());
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

    // ---- SchedulingSection (Story 5.1) ----

    #[test]
    fn scheduling_well_formed() {
        let s = SchedulingSection::from_toml_str(
            "priority_weight = 200\nyield_every_polls = 128\nidle_window_ms = 60000",
        )
        .unwrap();
        assert_eq!(s.priority_weight, 200);
        assert_eq!(s.yield_every_polls, 128);
        assert_eq!(s.idle_window_ms, 60000);
    }

    #[test]
    fn scheduling_malformed_priority_rejected() {
        let err = SchedulingSection::from_toml_str("priority_weight = 0").unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("scheduling.priority_weight")),
            "got: {err:?}"
        );
    }

    #[test]
    fn scheduling_malformed_priority_too_high() {
        // 256 overflows u8; TOML deser rejects it, not validate().
        let err = SchedulingSection::from_toml_str("priority_weight = 256").unwrap_err();
        assert!(matches!(&err, ManifestError::Toml(_)), "got: {err:?}");
    }

    #[test]
    fn scheduling_malformed_yield_too_low() {
        let err = SchedulingSection::from_toml_str("yield_every_polls = 0").unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("scheduling.yield_every_polls")),
            "got: {err:?}"
        );
    }

    #[test]
    fn scheduling_malformed_yield_too_high() {
        let err = SchedulingSection::from_toml_str("yield_every_polls = 4097").unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("scheduling.yield_every_polls")),
            "got: {err:?}"
        );
    }

    #[test]
    fn scheduling_malformed_idle_window_too_low() {
        let err = SchedulingSection::from_toml_str("idle_window_ms = 50").unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("scheduling.idle_window_ms")),
            "got: {err:?}"
        );
    }

    #[test]
    fn scheduling_malformed_idle_window_too_high() {
        let err =
            SchedulingSection::from_toml_str("idle_window_ms = 4000000").unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("scheduling.idle_window_ms")),
            "got: {err:?}"
        );
    }

    #[test]
    fn scheduling_edge_empty_defaults() {
        let s = SchedulingSection::from_toml_str("").unwrap();
        assert_eq!(s.priority_weight, 100);
        assert_eq!(s.yield_every_polls, 64);
        assert_eq!(s.idle_window_ms, 30000);
    }

    #[test]
    fn scheduling_default_trait() {
        let s = SchedulingSection::default();
        assert_eq!(s.priority_weight, 100);
        assert_eq!(s.yield_every_polls, 64);
        assert_eq!(s.idle_window_ms, 30000);
    }

    // ---- LifecycleSection (Story 5.1) ----

    #[test]
    fn lifecycle_well_formed() {
        let s = LifecycleSection::from_toml_str(
            r#"enabled_hooks = ["on_load", "on_start", "on_idle"]"#,
        )
        .unwrap();
        assert_eq!(s.enabled_hooks.len(), 3);
        assert!(s.enabled_hooks.contains(&"on_load".into()));
    }

    #[test]
    fn lifecycle_malformed_unknown_hook_rejected() {
        let err = LifecycleSection::from_toml_str(
            r#"enabled_hooks = ["on_load", "on_missing"]"#,
        )
        .unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("on_missing")),
            "got: {err:?}"
        );
    }

    #[test]
    fn lifecycle_malformed_duplicate_rejected() {
        let err = LifecycleSection::from_toml_str(
            r#"enabled_hooks = ["on_load", "on_load"]"#,
        )
        .unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("duplicate")),
            "got: {err:?}"
        );
    }

    #[test]
    fn lifecycle_edge_empty_means_all_allowed() {
        let s = LifecycleSection::from_toml_str("").unwrap();
        assert!(s.enabled_hooks.is_empty());
        // kernel_invocation_allowed(&[], _) → true
        assert!(maos_spirit_abi::lifecycle::kernel_invocation_allowed(
            &[],
            "on_load"
        ));
    }

    #[test]
    fn lifecycle_edge_all_11_hooks() {
        let hooks_toml = r#"enabled_hooks = [
            "on_load", "on_start", "on_frame", "on_idle",
            "on_telemetry_event", "on_schedule", "on_swap_in",
            "on_pause", "on_resume", "on_unload", "on_consolidate"
        ]"#;
        let s = LifecycleSection::from_toml_str(hooks_toml).unwrap();
        assert_eq!(s.enabled_hooks.len(), 11);
    }
}
