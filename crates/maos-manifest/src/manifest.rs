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
    /// Optional image-pin name referencing an entry in `t3-image.lock`.
    /// `None` uses the `default_for_v05 = true` entry.
    pub image_pin: Option<String>,
}

impl SandboxConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSandboxConfig =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        Ok(SandboxConfig {
            tier: raw.tier,
            image_pin: raw.image_pin,
        })
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
        let raw: RawResourceCaps =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
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
    #[serde(default)]
    image_pin: Option<String>,
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
                return Err(ManifestError::CapOutOfRange {
                    field: "cpu_max_pct".into(),
                    value: v,
                });
            }
            if v > 100 * num_cpus {
                return Err(ManifestError::CapOutOfRange {
                    field: "cpu_max_pct".into(),
                    value: v,
                });
            }
        }
        if let Some(v) = self.memory_max_mb {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange {
                    field: "memory_max_mb".into(),
                    value: v,
                });
            }
        }
        if let Some(v) = self.fd_max {
            if v == 0 {
                return Err(ManifestError::CapOutOfRange {
                    field: "fd_max".into(),
                    value: v,
                });
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
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
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

/// Schema sections added AFTER `manifest_schema_version = 1` (Epic 6 §A4 bump,
/// retro 2026-05-28): `[[cli_wrapper]]`, `[[schedule]]`, `[gateway]`; and the
/// schema-v3 bump (Story 9.4b AC-6): `[model_provenance]`. A manifest authored
/// at the N-1 floor omits these; they default via `#[serde(default)]` /
/// optional-on-read.
///
/// NOTE: kept on a single line via `rustfmt::skip`. The
/// `check-manifest-schema-version` xtask gate parses this constant with a
/// single-line matcher; without the skip, rustfmt wraps it at 101 cols and the
/// gate reports the constant as missing. Keep the declaration on one physical
/// line and out of doc comments above it.
#[rustfmt::skip]
const POST_V1_SCHEMA_SECTIONS: &[&str] = &["cli_wrapper", "schedule", "gateway", "model_provenance", "capabilities.required.loom"];

/// Story 7.5a (NFR-Maint-9) — emit a WARN-level degradation note for every
/// newer-than-declared schema section that an N-1 manifest omits (and thus
/// defaults). Returns the list of degraded section names (empty when the
/// declared schema is current/ahead — nothing degrades).
///
/// This is the kernel's "documented degradation paths emitted to tracing at
/// WARN" commitment in operational form: the operator sees, per defaulted
/// section, exactly which newer surface the V-1 manifest predates. Called from
/// the admission chokepoint (`SecurityManagerAdapter::admit_spirit`) AND
/// exercised directly by the manifest N-1 field-coverage test, so the WARN
/// path is the SAME code in production and test.
///
/// Fail-closed boundaries (`< MIN_SUPPORTED` / `> MAX_SUPPORTED`) are NOT this
/// function's concern — they are rejected with typed `EAbiTooOld`/`EAbiTooNew`
/// before this is reached. This only handles the *supported* N-1 degradation.
pub fn warn_n_minus_1_degradations(declared_schema: u32) -> Vec<&'static str> {
    if declared_schema >= maos_spirit_abi::MANIFEST_SCHEMA_VERSION {
        return Vec::new();
    }
    for section in POST_V1_SCHEMA_SECTIONS {
        tracing::warn!(
            declared_schema,
            current_schema = maos_spirit_abi::MANIFEST_SCHEMA_VERSION,
            section = *section,
            "manifest N-1 degradation: section '[{section}]' absent in V-{} manifest — defaulted via serde(default)",
            maos_spirit_abi::MANIFEST_SCHEMA_VERSION - declared_schema,
        );
    }
    POST_V1_SCHEMA_SECTIONS.to_vec()
}

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
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
        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ManifestError::Toml(validation_msg(
                "class.name",
                "must be [a-z0-9-] only",
            )));
        }
        // version: semver-ish (best-effort major.minor.patch).
        if self.version.split('.').count() < 3 {
            return Err(ManifestError::Toml(validation_msg(
                "class.version",
                &format!("not semver: {}", self.version),
            )));
        }
        // abi: format `<major>.<minor>`; v0.1-β only accepts `"1.0"`.
        if self.abi != "1.0" {
            return Err(ManifestError::Toml(validation_msg(
                "class.abi",
                &format!("v0.1-β only accepts \"1.0\", got {}", self.abi),
            )));
        }
        // manifest_schema_version: must fall within the kernel's accepted
        // range. The bounds are sourced from `maos-spirit-abi` so the kernel
        // build, the manifest validator, and the `xtask
        // check-manifest-schema-version` gate all agree on the window.
        //
        // Epic 6 §A4 (retro 2026-05-28) bumped MAX from 1 → 2 to track the
        // four additive sections landed across Stories 6.2 / 6.4 / 6.5:
        // `[[cli_wrapper]]`, `[[schedules]]`, `[gateways]`, plus the
        // ConsentEnvelope.intent_class + valid_until_ns additive fields.
        // MIN remains at 1 — v1 (Epic 1b baseline) manifests still load.
        const MIN: u32 = maos_spirit_abi::MIN_SUPPORTED_MANIFEST_SCHEMA_VERSION;
        const MAX: u32 = maos_spirit_abi::MAX_SUPPORTED_MANIFEST_SCHEMA_VERSION;
        if self.manifest_schema_version < MIN || self.manifest_schema_version > MAX {
            return Err(ManifestError::Toml(validation_msg(
                "class.manifest_schema_version",
                &format!(
                    "kernel accepts {MIN}..={MAX}, got {}",
                    self.manifest_schema_version
                ),
            )));
        }
        // min_substrate_version: semver with numeric parts (allow pre-release suffix).
        // Each dot-separated segment (before any '-' pre-release) must start with
        // an ASCII digit — rejects wildcards like "*.*.*" that would bypass the
        // substrate version gate via lexicographic comparison.
        let prefix = self
            .min_substrate_version
            .split('-')
            .next()
            .unwrap_or_default();
        if prefix.split('.').count() < 3 {
            return Err(ManifestError::Toml(validation_msg(
                "class.min_substrate_version",
                &format!("not semver: {}", self.min_substrate_version),
            )));
        }
        for segment in prefix.split('.') {
            if segment.is_empty() || !segment.bytes().next().map_or(false, |b| b.is_ascii_digit()) {
                return Err(ManifestError::Toml(validation_msg(
                    "class.min_substrate_version",
                    &format!(
                        "non-numeric segment '{}' in {}",
                        segment, self.min_substrate_version
                    ),
                )));
            }
        }
        // forms: non-empty + every value ∈ {rust-inproc, subprocess}.
        if self.forms.is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "class.forms",
                "must be non-empty",
            )));
        }
        for f in &self.forms {
            if !matches!(f.as_str(), "rust-inproc" | "subprocess") {
                return Err(ManifestError::Toml(validation_msg(
                    "class.forms",
                    &format!("unknown form: {f}"),
                )));
            }
        }
        // trust_tier ∈ {local, org-internal, public-untrusted}.
        if !matches!(
            self.trust_tier.as_str(),
            "local" | "org-internal" | "public-untrusted"
        ) {
            return Err(ManifestError::Toml(validation_msg(
                "class.trust_tier",
                &format!("unknown trust_tier: {}", self.trust_tier),
            )));
        }
        // description: non-empty + ≤4 KiB.
        if self.description.is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "class.description",
                "empty",
            )));
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
    pub mcp: McpCapabilities,
    pub loom: LoomCapabilities,
}

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub complete: Vec<String>,
}

/// Story 13.5d — declared collective capabilities. Scopes are unit variants,
/// so this is intentionally three explicit booleans rather than a false
/// namespace-qualified promise.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LoomCapabilities {
    pub read: bool,
    pub write: bool,
    pub scan: bool,
}

/// Story 5.5c — MCP server/tool capability declarations.
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCapabilities {
    pub servers: Vec<McpCapabilityServerEntry>,
}

/// Story 5.5c — a single MCP server with its allowed tools.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpCapabilityServerEntry {
    pub name: String,
    pub allowed_tools: Vec<String>,
}

impl CapabilitiesRequired {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawCapabilitiesRequired =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
    /// Drops capability declarations introduced after the manifest's schema.
    ///
    /// Schema v4 introduced `[capabilities.required.loom]`; older manifests
    /// degrade it rather than gaining a capability their declared schema cannot
    /// express.
    pub fn degrade_for_schema_version(mut self, declared_schema: u32) -> Self {
        if declared_schema < 4 {
            self.loom = LoomCapabilities::default();
        }
        self
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapabilitiesRequired {
    provider: RawProviderCapabilities,
    #[serde(default)]
    mcp: RawMcpCapabilities,
    #[serde(default)]
    loom: RawLoomCapabilities,
}

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProviderCapabilities {
    complete: Vec<String>,
}

/// Story 5.5c — raw MCP capability section from TOML.
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawMcpCapabilities {
    #[serde(default)]
    servers: Vec<RawMcpCapabilityServerEntry>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RawLoomCapabilities {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
    #[serde(default)]
    scan: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcpCapabilityServerEntry {
    name: String,
    allowed_tools: Vec<String>,
}

impl RawCapabilitiesRequired {
    fn validate(self) -> Result<CapabilitiesRequired, ManifestError> {
        if self.provider.complete.is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "capabilities.required.provider.complete",
                "must be non-empty",
            )));
        }
        for v in &self.provider.complete {
            if v.len() > 128 {
                return Err(ManifestError::Toml(validation_msg(
                    "capabilities.required.provider.complete",
                    &format!("entry > 128 chars: {v}"),
                )));
            }
        }
        Ok(CapabilitiesRequired {
            provider: ProviderCapabilities {
                complete: self.provider.complete,
            },
            mcp: McpCapabilities {
                servers: self
                    .mcp
                    .servers
                    .into_iter()
                    .map(|s| McpCapabilityServerEntry {
                        name: s.name,
                        allowed_tools: s.allowed_tools,
                    })
                    .collect(),
            },
            loom: LoomCapabilities {
                read: self.loom.read,
                write: self.loom.write,
                scan: self.loom.scan,
            },
        })
    }
}

/// Convert a `CapabilitiesRequired` manifest declaration to a `Vec<Scope>`.
///
/// v0.1-β produces `Scope::ProviderInfer` from provider entries and
/// `Scope::McpCall` from MCP server/tool entries (Story 5.5c).
pub fn capabilities_required_to_scopes(
    caps: &CapabilitiesRequired,
) -> Vec<maos_domain::invariants::i1::Scope> {
    let mut scopes: Vec<maos_domain::invariants::i1::Scope> = caps
        .provider
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
        .collect();
    for server_entry in &caps.mcp.servers {
        for tool in &server_entry.allowed_tools {
            scopes.push(maos_domain::invariants::i1::Scope::McpCall {
                server: server_entry.name.clone(),
                tool: tool.clone(),
            });
        }
    }
    if caps.loom.read {
        scopes.push(maos_domain::invariants::i1::Scope::LoomRead);
    }
    if caps.loom.write {
        scopes.push(maos_domain::invariants::i1::Scope::LoomWrite);
    }
    if caps.loom.scan {
        scopes.push(maos_domain::invariants::i1::Scope::LoomScan);
    }
    scopes
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
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
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

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutputShape {
    required_fields: Vec<String>,
}

impl RawOutputShape {
    fn validate(self) -> Result<OutputShape, ManifestError> {
        if self.required_fields.is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "output_shape.required_fields",
                "must be non-empty",
            )));
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
#[maos_attrs::i9_exempt(
    reason = "manifest-derived predicate; constructed at admission and held in SandboxSpec — dropped after spawn, no kernel persistence"
)]
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
        let raw: RawBudget = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
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
            return Err(ManifestError::Toml(validation_msg(
                "budget.context_window_size",
                "must be > 0",
            )));
        }
        if self.context_window_size > (1u32 << 24) {
            return Err(ManifestError::Toml(validation_msg(
                "budget.context_window_size",
                &format!("{} > 2^24", self.context_window_size),
            )));
        }
        if self.time_cap_seconds == 0 {
            return Err(ManifestError::Toml(validation_msg(
                "budget.time_cap_seconds",
                "must be > 0",
            )));
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
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
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

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
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
                &format!("lower ({}) > upper ({})", raw.lower, raw.upper),
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
                &format!("lower ({}) > upper ({})", raw.lower, raw.upper),
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
                &format!("must be in [1, 255], got {}", self.priority_weight),
            )));
        }
        if self.yield_every_polls < 1 || self.yield_every_polls > 4096 {
            return Err(ManifestError::Toml(validation_msg(
                "scheduling.yield_every_polls",
                &format!("must be in [1, 4096], got {}", self.yield_every_polls),
            )));
        }
        if self.idle_window_ms < 100 || self.idle_window_ms > 3_600_000 {
            return Err(ManifestError::Toml(validation_msg(
                "scheduling.idle_window_ms",
                &format!("must be in [100, 3600000], got {}", self.idle_window_ms),
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
// [on_crash] section (Story 5.3, AC5)
// ------------------------------------------------------------------

/// The `[on_crash]` manifest section — dead-Spirit task disposition policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OnCrashSection {
    pub action: maos_domain::supervision::OnCrashAction,
}

impl OnCrashSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawOnCrashSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOnCrashSection {
    #[serde(default)]
    action: String,
}

impl RawOnCrashSection {
    fn validate(self) -> Result<OnCrashSection, ManifestError> {
        let action = match self.action.as_str() {
            "" | "nack" => maos_domain::supervision::OnCrashAction::Nack,
            "reassign-to-replica" => maos_domain::supervision::OnCrashAction::ReassignToReplica,
            "escalate-to-operator" => maos_domain::supervision::OnCrashAction::EscalateToOperator,
            other => {
                return Err(ManifestError::Toml(validation_msg(
                    "on_crash.action",
                    &format!("unknown value '{}'", other),
                )));
            }
        };
        Ok(OnCrashSection { action })
    }
}

// ------------------------------------------------------------------
// [on_revocation] section (Story 5.4, AC5)
// ------------------------------------------------------------------

/// The `[on_revocation]` manifest section — revocation policy for this Spirit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnRevocationSection {
    pub action: maos_domain::revocation::RevocationAction,
}

impl OnRevocationSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawOnRevocationSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOnRevocationSection {
    #[serde(default)]
    action: String,
}

impl RawOnRevocationSection {
    fn validate(self) -> Result<OnRevocationSection, ManifestError> {
        use maos_domain::revocation::RevocationAction;
        let action = match self.action.as_str() {
            "" | "terminate-immediately" => RevocationAction::TerminateImmediately,
            "drain-then-terminate" => RevocationAction::DrainThenTerminate,
            "quarantine" => RevocationAction::Quarantine,
            other => {
                return Err(ManifestError::Toml(validation_msg(
                    "on_revocation.action",
                    &format!("unknown value '{}'", other),
                )));
            }
        };
        Ok(OnRevocationSection { action })
    }
}

// ------------------------------------------------------------------
// [[schedule]] section (Story 6.4 / FR26 / ADR-025)
// ------------------------------------------------------------------

/// Story 6.4 / FR26 — `[[schedule]]` manifest entry.
///
/// Each entry declares one scheduled invocation that fires
/// `on_schedule(ctx, schedule_id, payload)` at the declared cadence. ADR-025
/// governs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleEntry {
    /// Unique id within this manifest. `[a-zA-Z0-9_-]{1,64}`.
    pub id: String,
    /// Firing cadence in seconds. Range `[1, 604_800]` (1s to 1 week).
    pub cadence_secs: u32,
    /// Opaque payload bytes delivered to `on_schedule`. Decoded from
    /// `payload_b64` at parse time; empty when absent.
    pub payload_bytes: Vec<u8>,
    /// Per-schedule rate-limit (firing cap regardless of cadence). Range
    /// `[1, 3600]` per ADR-025's ≥1s cadence floor.
    pub rate_limit_per_hour: u32,
    /// ComplianceClaim envelope reference (Story 7.3). At v0.5 a structural
    /// pass-through into the TL row; Story 7.3 lands the cryptographic verify.
    pub compliance_claim_ref: Option<[u8; 32]>,
    /// When true, a revoked principal halts the firing.
    pub principal_revocability: bool,
    /// Side-effect allowlist — `Scope` subset granted to the per-firing
    /// cap-token. Empty = memory-only.
    pub side_effect_scopes: Vec<maos_domain::invariants::i1::Scope>,
}

/// The `[[schedule]]` section — `Vec<ScheduleEntry>` with cross-entry id
/// uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchedulesSection {
    pub entries: Vec<ScheduleEntry>,
}

impl SchedulesSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSchedulesSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSchedulesSection {
    #[serde(default, rename = "schedule")]
    entries: Vec<RawScheduleEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScheduleEntry {
    id: String,
    cadence_secs: u32,
    #[serde(default)]
    payload_b64: String,
    #[serde(default = "default_rate_limit_per_hour")]
    rate_limit_per_hour: u32,
    #[serde(default)]
    compliance_claim_ref_hex: Option<String>,
    #[serde(default = "default_principal_revocability")]
    principal_revocability: bool,
    #[serde(default)]
    side_effect_scopes: Vec<maos_domain::invariants::i1::Scope>,
}

fn default_rate_limit_per_hour() -> u32 {
    60
}
fn default_principal_revocability() -> bool {
    true
}

impl RawSchedulesSection {
    fn validate(self) -> Result<SchedulesSection, ManifestError> {
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        for raw in self.entries {
            // id shape — non-empty, [a-zA-Z0-9_-]{1,64}
            if raw.id.is_empty() || raw.id.len() > 64 {
                return Err(ManifestError::Toml(validation_msg(
                    "schedule.id",
                    &format!("must be 1..=64 chars, got len {}", raw.id.len()),
                )));
            }
            if !raw
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(ManifestError::Toml(validation_msg(
                    "schedule.id",
                    &format!("must match [a-zA-Z0-9_-]+, got '{}'", raw.id),
                )));
            }
            if !seen_ids.insert(raw.id.clone()) {
                return Err(ManifestError::Toml(validation_msg(
                    "schedule.id",
                    &format!("duplicate schedule id '{}'", raw.id),
                )));
            }
            if raw.cadence_secs < 1 || raw.cadence_secs > 604_800 {
                return Err(ManifestError::Toml(validation_msg(
                    "schedule.cadence_secs",
                    &format!("must be in [1, 604_800], got {}", raw.cadence_secs),
                )));
            }
            if raw.rate_limit_per_hour < 1 || raw.rate_limit_per_hour > 3600 {
                return Err(ManifestError::Toml(validation_msg(
                    "schedule.rate_limit_per_hour",
                    &format!("must be in [1, 3600], got {}", raw.rate_limit_per_hour),
                )));
            }
            // payload_b64 decode (base64 standard). Hand-rolled minimal decoder
            // — workspace has no `base64` dep; FR47 forbids adding one for
            // Story 6.4 (verified via xtask). Empty string → empty bytes.
            let payload_bytes = decode_b64_strict(&raw.payload_b64)
                .map_err(|e| ManifestError::Toml(validation_msg("schedule.payload_b64", &e)))?;
            // compliance_claim_ref_hex — 64 hex chars (= 32 bytes) if present.
            let compliance_claim_ref = match raw.compliance_claim_ref_hex {
                None => None,
                Some(hex) => {
                    // Accept `sha256:<hex>` prefix or raw hex.
                    let stripped = hex.strip_prefix("sha256:").unwrap_or(hex.as_str());
                    if stripped.len() != 64 {
                        return Err(ManifestError::Toml(validation_msg(
                            "schedule.compliance_claim_ref_hex",
                            &format!(
                                "must be 64 hex chars (32 bytes), got {} chars",
                                stripped.len()
                            ),
                        )));
                    }
                    let mut buf = [0u8; 32];
                    for (i, byte) in buf.iter_mut().enumerate() {
                        let s = &stripped[i * 2..i * 2 + 2];
                        *byte = u8::from_str_radix(s, 16).map_err(|_| {
                            ManifestError::Toml(validation_msg(
                                "schedule.compliance_claim_ref_hex",
                                &format!("non-hex char in '{}'", s),
                            ))
                        })?;
                    }
                    Some(buf)
                }
            };
            entries.push(ScheduleEntry {
                id: raw.id,
                cadence_secs: raw.cadence_secs,
                payload_bytes,
                rate_limit_per_hour: raw.rate_limit_per_hour,
                compliance_claim_ref,
                principal_revocability: raw.principal_revocability,
                side_effect_scopes: raw.side_effect_scopes,
            });
        }
        Ok(SchedulesSection { entries })
    }
}

/// Minimal base64 (RFC 4648 §4 standard alphabet) decoder for the
/// `[[schedule]].payload_b64` field. We avoid pulling a new workspace
/// dependency (FR47 posture); the alphabet table is constant and the loop
/// is straight-line. Padding (`=`) is required to complete the final group.
fn decode_b64_strict(input: &str) -> Result<Vec<u8>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(format!("length {} is not a multiple of 4", bytes.len()));
    }
    fn decode_char(c: u8) -> Result<u8, String> {
        Ok(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(format!("non-base64 char '{}'", c as char)),
        })
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks_exact(4) {
        let pad_count = chunk.iter().rev().take_while(|&&b| b == b'=').count();
        if pad_count > 2 {
            return Err("more than 2 padding chars in a group".into());
        }
        // Reject padding in non-trailing positions (e.g., "A=BC").
        let mut vals = [0u8; 4];
        let mut pad_seen = false;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                pad_seen = true;
                vals[i] = 0;
            } else {
                if pad_seen {
                    return Err("padding character '=' in non-trailing position".into());
                }
                vals[i] = decode_char(c)?;
            }
        }
        let triplet = (u32::from(vals[0]) << 18)
            | (u32::from(vals[1]) << 12)
            | (u32::from(vals[2]) << 6)
            | u32::from(vals[3]);
        out.push((triplet >> 16) as u8);
        if pad_count < 2 {
            out.push((triplet >> 8) as u8);
        }
        if pad_count < 1 {
            out.push(triplet as u8);
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------
// [supervision] section (Story 5.3, AC2 + AC3)
// ------------------------------------------------------------------

/// The `[supervision]` manifest section — watchdog tuning parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisionSection {
    pub heartbeat_interval_ms: u32,
    pub progress_threshold_ms: u32,
    pub silent_failure_threshold_ms: u32,
}

impl Default for SupervisionSection {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 5000,
            progress_threshold_ms: 30000,
            silent_failure_threshold_ms: 30000,
        }
    }
}

impl SupervisionSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawSupervisionSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSupervisionSection {
    #[serde(default = "default_heartbeat_interval_ms")]
    heartbeat_interval_ms: u32,
    #[serde(default = "default_progress_threshold_ms")]
    progress_threshold_ms: u32,
    #[serde(default = "default_silent_failure_threshold_ms")]
    silent_failure_threshold_ms: u32,
}

fn default_heartbeat_interval_ms() -> u32 {
    5000
}
fn default_progress_threshold_ms() -> u32 {
    30000
}
fn default_silent_failure_threshold_ms() -> u32 {
    30000
}

impl RawSupervisionSection {
    fn validate(self) -> Result<SupervisionSection, ManifestError> {
        if self.heartbeat_interval_ms < 1000 || self.heartbeat_interval_ms > 60000 {
            return Err(ManifestError::Toml(validation_msg(
                "supervision.heartbeat_interval_ms",
                &format!(
                    "must be in [1000, 60000], got {}",
                    self.heartbeat_interval_ms
                ),
            )));
        }
        if self.progress_threshold_ms < 5000 || self.progress_threshold_ms > 300000 {
            return Err(ManifestError::Toml(validation_msg(
                "supervision.progress_threshold_ms",
                &format!(
                    "must be in [5000, 300000], got {}",
                    self.progress_threshold_ms
                ),
            )));
        }
        if self.silent_failure_threshold_ms < 5000 || self.silent_failure_threshold_ms > 300000 {
            return Err(ManifestError::Toml(validation_msg(
                "supervision.silent_failure_threshold_ms",
                &format!(
                    "must be in [5000, 300000], got {}",
                    self.silent_failure_threshold_ms
                ),
            )));
        }
        Ok(SupervisionSection {
            heartbeat_interval_ms: self.heartbeat_interval_ms,
            progress_threshold_ms: self.progress_threshold_ms,
            silent_failure_threshold_ms: self.silent_failure_threshold_ms,
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
        let raw: RawAuthor = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
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
                return Err(ManifestError::Toml(validation_msg(
                    "author.homepage",
                    &format!("must start with http:// or https://, got: {h}"),
                )));
            }
        }
        Ok(Author {
            name: self.name,
            homepage: self.homepage,
        })
    }
}

// ------------------------------------------------------------------
// [model_provenance] section (Story 9.4b AC-6, NFR-Comp-5 / SB-1047)
// ------------------------------------------------------------------

/// The `[model_provenance]` manifest section — SB-1047 model provenance
/// (D5/D6). **Optional on read** (AC-11): pre-v3 manifests omit it and stay
/// admissible. `training_data_lineage` is **schema-constrained to reverse-DNS
/// lineage references — NOT free-text** (D5), which makes "zero principal
/// nexus" structural rather than promised (pasted prose / PII cannot satisfy
/// the grammar).
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProvenanceSection {
    pub covered_model_id: String,
    pub training_data_lineage: Vec<String>,
    pub last_eval_timestamp: String,
    /// `last_eval_timestamp` parsed to Unix seconds (UTC) at validation time;
    /// the admission staleness check (D6) compares against this.
    pub last_eval_unix_secs: i64,
}

impl ModelProvenanceSection {
    /// Parse a standalone `[model_provenance]` section body.
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawModelProvenanceSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }

    /// Extract the OPTIONAL `[model_provenance]` table from a full manifest TOML
    /// document. Returns `Ok(None)` when absent (AC-11 — pre-v3 manifests stay
    /// admissible), `Ok(Some(_))` when present-and-valid, `Err` when present but
    /// malformed.
    pub fn from_manifest_toml(full_toml: &str) -> Result<Option<Self>, ManifestError> {
        let root: toml::Value =
            toml::from_str(full_toml).map_err(|e| ManifestError::Toml(e.to_string()))?;
        match root.get("model_provenance") {
            None => Ok(None),
            Some(v) => {
                let section_toml =
                    toml::to_string(v).map_err(|e| ManifestError::Toml(e.to_string()))?;
                Self::from_toml_str(&section_toml).map(Some)
            }
        }
    }

    /// AC-6 / D6 — admission staleness policy. Returns `EModelProvenanceStale`
    /// when the declared evaluation is older than `max_age_secs` relative to
    /// `now_unix_secs`.
    pub fn validate_staleness(
        &self,
        now_unix_secs: i64,
        max_age_secs: u64,
    ) -> Result<(), maos_domain::provenance::ProvenanceError> {
        let age = now_unix_secs.saturating_sub(self.last_eval_unix_secs);
        if age < 0 {
            // Clock skew or future eval timestamp — not stale.
            return Ok(());
        }
        if (age as u64) > max_age_secs {
            return Err(
                maos_domain::provenance::ProvenanceError::EModelProvenanceStale {
                    last_eval_unix_secs: self.last_eval_unix_secs,
                    now_unix_secs,
                    max_age_secs,
                },
            );
        }
        Ok(())
    }

    /// Canonical content bytes for the FR62 governance content-hash (D6). A
    /// stable, field-ordered, length-prefixed serialization of the provenance
    /// triple — NOT the raw TOML (which varies with formatting/whitespace), so
    /// the content hash is reproducible across re-admissions.
    pub fn canonical_content_bytes(&self) -> Vec<u8> {
        fn push_lp(out: &mut Vec<u8>, s: &str) {
            out.extend_from_slice(&(s.len() as u64).to_le_bytes());
            out.extend_from_slice(s.as_bytes());
        }
        let mut out = Vec::new();
        push_lp(&mut out, &self.covered_model_id);
        push_lp(&mut out, &self.last_eval_timestamp);
        out.extend_from_slice(&(self.training_data_lineage.len() as u64).to_le_bytes());
        for l in &self.training_data_lineage {
            push_lp(&mut out, l);
        }
        out
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawModelProvenanceSection {
    covered_model_id: String,
    training_data_lineage: Vec<String>,
    last_eval_timestamp: String,
}

impl RawModelProvenanceSection {
    fn validate(self) -> Result<ModelProvenanceSection, ManifestError> {
        if self.covered_model_id.trim().is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "model_provenance.covered_model_id",
                "empty",
            )));
        }
        if self.training_data_lineage.is_empty() {
            return Err(ManifestError::Toml(validation_msg(
                "model_provenance.training_data_lineage",
                "must list at least one reverse-DNS lineage reference (free-text is rejected — D5)",
            )));
        }
        for (i, l) in self.training_data_lineage.iter().enumerate() {
            if !is_reverse_dns_lineage(l) {
                return Err(ManifestError::Toml(validation_msg(
                    &format!("model_provenance.training_data_lineage[{i}]"),
                    &format!(
                        "'{l}' is not a structured reverse-DNS lineage reference \
                         (NOT free-text) — D5"
                    ),
                )));
            }
        }
        let last_eval_unix_secs =
            parse_rfc3339_utc_secs(&self.last_eval_timestamp).map_err(|e| {
                ManifestError::Toml(validation_msg("model_provenance.last_eval_timestamp", &e))
            })?;
        Ok(ModelProvenanceSection {
            covered_model_id: self.covered_model_id,
            training_data_lineage: self.training_data_lineage,
            last_eval_timestamp: self.last_eval_timestamp,
            last_eval_unix_secs,
        })
    }
}

/// A structured reverse-DNS lineage reference (D5): two or more dot-separated
/// labels, each non-empty `[a-z0-9-]` (lowercase), no leading/trailing/double
/// dots. This grammar is what makes `training_data_lineage` NON-free-text —
/// pasted prose or PII cannot satisfy it, so "zero principal nexus" is
/// structural.
fn is_reverse_dns_lineage(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }
    let labels: Vec<&str> = s.split('.').collect();
    if labels.len() < 2 {
        return false;
    }
    labels.iter().all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Minimal strict RFC3339 (UTC `Z`) → Unix seconds. Accepts exactly
/// `YYYY-MM-DDTHH:MM:SSZ` (no offsets, no fractional seconds) so that v1.0
/// model-provenance timestamps stay unambiguous and dependency-free. Returns a
/// human-readable reason on malformed input.
fn parse_rfc3339_utc_secs(s: &str) -> Result<i64, String> {
    let b = s.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return Err(format!(
            "expected strict UTC RFC3339 'YYYY-MM-DDTHH:MM:SSZ', got '{s}'"
        ));
    }
    let num = |a: usize, z: usize| -> Result<i64, String> {
        std::str::from_utf8(&b[a..z])
            .ok()
            .filter(|x| x.chars().all(|c| c.is_ascii_digit()))
            .and_then(|x| x.parse::<i64>().ok())
            .ok_or_else(|| format!("non-numeric field in timestamp '{s}'"))
    };
    let year = num(0, 4)?;
    let month = num(5, 7)?;
    let day = num(8, 10)?;
    let hour = num(11, 13)?;
    let min = num(14, 16)?;
    let sec = num(17, 19)?;
    if !(1..=12).contains(&month) {
        return Err(format!("month out of range in '{s}'"));
    }
    if hour > 23 || min > 59 || sec > 60 {
        return Err(format!("time field out of range in '{s}'"));
    }
    let mdays = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > mdays[(month - 1) as usize] {
        return Err(format!("day out of range in '{s}'"));
    }
    let mut days: i64 = 0;
    if year >= 1970 {
        for y in 1970..year {
            days += if is_leap_year(y) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if is_leap_year(y) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += mdays[(m - 1) as usize];
    }
    days += day - 1;
    Ok(days * 86_400 + hour * 3_600 + min * 60 + sec)
}

// ------------------------------------------------------------------
// [providers] section (Story 5.5b, AC2)
// ------------------------------------------------------------------

const ALLOWED_PROVIDER_IDS: &[&str] = &["anthropic", "openai", "ollama"];

fn is_hex_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvidersSection {
    #[doc = "Construct via [`ProvidersSection::new`] to enforce id/endpoint validation; struct literals bypass schema checks."]
    pub primary: ProviderConfig,
    #[doc = "Construct via [`ProvidersSection::new`] to enforce id/endpoint validation; struct literals bypass schema checks."]
    pub fallback: Vec<ProviderConfig>,
}

impl ProvidersSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawProvidersSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }

    pub fn new(primary: ProviderConfig, fallback: Vec<ProviderConfig>) -> Self {
        Self { primary, fallback }
    }
}

#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
    pub id: String,
    #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
    pub endpoint_url: Option<String>,
    #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
    pub model_id: Option<String>,
    #[doc = "Construct via [`ProviderConfig::new`] to enforce non-empty id."]
    pub provider_endpoint_pin: Option<String>,
}

impl ProviderConfig {
    pub fn new(
        id: String,
        endpoint_url: Option<String>,
        model_id: Option<String>,
        provider_endpoint_pin: Option<String>,
    ) -> Self {
        Self {
            id,
            endpoint_url,
            model_id,
            provider_endpoint_pin,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProvidersSection {
    primary: RawProviderConfig,
    #[serde(default)]
    fallback: Vec<RawProviderConfig>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawProviderConfig {
    id: String,
    endpoint_url: Option<String>,
    model_id: Option<String>,
    provider_endpoint_pin: Option<String>,
}

fn validate_provider_config(
    raw: RawProviderConfig,
    prefix: &str,
) -> Result<ProviderConfig, ManifestError> {
    if !ALLOWED_PROVIDER_IDS.contains(&raw.id.as_str()) {
        return Err(ManifestError::Toml(format!(
            "providers.{}.id '{}' unsupported at v0.5-α (allowed: {})",
            prefix,
            raw.id,
            ALLOWED_PROVIDER_IDS.join(", ")
        )));
    }
    if let Some(ref url) = raw.endpoint_url {
        if url.is_empty() {
            return Err(ManifestError::Toml(format!(
                "providers.{}.endpoint_url must not be empty if present",
                prefix
            )));
        }
    }
    if let Some(ref pin) = raw.provider_endpoint_pin {
        if !is_hex_sha256(pin) {
            return Err(ManifestError::Toml(format!(
                "providers.{}.provider_endpoint_pin must be 64-char hex SHA-256",
                prefix
            )));
        }
    }
    Ok(ProviderConfig {
        id: raw.id,
        endpoint_url: raw.endpoint_url,
        model_id: raw.model_id,
        provider_endpoint_pin: raw.provider_endpoint_pin,
    })
}

impl RawProvidersSection {
    fn validate(self) -> Result<ProvidersSection, ManifestError> {
        let primary = validate_provider_config(self.primary, "primary")?;
        let mut fallback = Vec::with_capacity(self.fallback.len());
        for (i, raw_fb) in self.fallback.into_iter().enumerate() {
            let cfg = validate_provider_config(raw_fb, &format!("fallback[{}]", i))?;
            fallback.push(cfg);
        }
        Ok(ProvidersSection { primary, fallback })
    }
}

// ------------------------------------------------------------------
// [mcp] section (Story 5.5c)
// ------------------------------------------------------------------

/// Story 5.5c — MCP tool-server declarations for the Spirit manifest.
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSection {
    #[doc = "Construct via [`McpSection::new`] to enforce uniqueness of server.name across the section."]
    pub servers: Vec<McpServerEntry>,
}

impl McpSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawMcpSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }

    pub fn new(servers: Vec<McpServerEntry>) -> Self {
        Self { servers }
    }
}

/// A single MCP server entry in the `[mcp].servers` manifest table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerEntry {
    #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
    pub name: String,
    #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
    pub uri: String,
    pub transport: maos_domain::ports::mcp::McpTransportId,
    #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
    pub fallback_transport: Option<maos_domain::ports::mcp::McpTransportId>,
    pub server_trust_tier: maos_domain::ports::registry::TrustTier,
    #[doc = "Construct via [`McpServerEntry::new`] to enforce non-empty name + non-empty uri + non-empty allowed_tools."]
    pub allowed_tools: Vec<String>,
}

impl McpServerEntry {
    pub fn new(
        name: String,
        uri: String,
        transport: maos_domain::ports::mcp::McpTransportId,
        fallback_transport: Option<maos_domain::ports::mcp::McpTransportId>,
        server_trust_tier: maos_domain::ports::registry::TrustTier,
        allowed_tools: Vec<String>,
    ) -> Self {
        Self {
            name,
            uri,
            transport,
            fallback_transport,
            server_trust_tier,
            allowed_tools,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcpSection {
    #[serde(default)]
    servers: Vec<RawMcpServerEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMcpServerEntry {
    name: String,
    uri: String,
    transport: maos_domain::ports::mcp::McpTransportId,
    #[serde(default)]
    fallback_transport: Option<maos_domain::ports::mcp::McpTransportId>,
    #[serde(default = "default_server_trust_tier")]
    server_trust_tier: maos_domain::ports::registry::TrustTier,
    #[serde(default)]
    allowed_tools: Vec<String>,
}

fn default_server_trust_tier() -> maos_domain::ports::registry::TrustTier {
    maos_domain::ports::registry::TrustTier::PublicUntrusted
}

impl RawMcpSection {
    fn validate(self) -> Result<McpSection, ManifestError> {
        let mut seen_names = std::collections::BTreeSet::new();
        let mut entries = Vec::with_capacity(self.servers.len());

        for (i, raw) in self.servers.into_iter().enumerate() {
            // Reject empty name
            if raw.name.trim().is_empty() {
                return Err(ManifestError::Toml(format!(
                    "mcp.servers[{i}].name must not be empty"
                )));
            }
            // Reject duplicate names
            if !seen_names.insert(raw.name.clone()) {
                return Err(ManifestError::Toml(format!(
                    "mcp.servers[{i}].name '{}' is a duplicate",
                    raw.name
                )));
            }
            // Reject empty URI
            if raw.uri.trim().is_empty() {
                return Err(ManifestError::Toml(format!(
                    "mcp.servers[{i}].uri must not be empty"
                )));
            }
            // Reject fallback == primary
            if let Some(ref fb) = raw.fallback_transport {
                if fb == &raw.transport {
                    return Err(ManifestError::Toml(format!(
                        "mcp.servers[{i}].fallback_transport must differ from primary"
                    )));
                }
            }
            // Reject public-vetted trust tier (FR37 exclusion)
            if raw.server_trust_tier == maos_domain::ports::registry::TrustTier::PublicVetted {
                return Err(ManifestError::Toml(format!(
                    "mcp.servers[{i}].server_trust_tier 'public_vetted' deferred per FR37 to v2.5; allowed: local, org_internal, public_untrusted"
                )));
            }
            // Reject empty or glob allowed_tools
            if raw.allowed_tools.is_empty() {
                return Err(ManifestError::Toml(format!(
                    "mcp.servers[{i}].allowed_tools must not be empty"
                )));
            }
            for (j, tool) in raw.allowed_tools.iter().enumerate() {
                if tool.trim().is_empty() || tool.contains('*') {
                    return Err(ManifestError::Toml(format!(
                        "mcp.servers[{i}].allowed_tools[{j}] '{}' — empty-or-glob, explicit tool names only at v0.5-α",
                        tool
                    )));
                }
            }

            entries.push(McpServerEntry::new(
                raw.name,
                raw.uri,
                raw.transport,
                raw.fallback_transport,
                raw.server_trust_tier,
                raw.allowed_tools,
            ));
        }

        Ok(McpSection::new(entries))
    }
}

// ------------------------------------------------------------------
// Tests (NFR-Test-13 &ge;3 cases per field)
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
        let caps =
            ResourceCaps::from_toml_str("cpu_max_pct = 50\nmemory_max_mb = 512\nfd_max = 64")
                .unwrap();
        assert_eq!(caps.cpu_max_pct, Some(50));
        assert_eq!(caps.memory_max_mb, Some(512));
        assert_eq!(caps.fd_max, Some(64));
    }

    #[test]
    fn resource_caps_malformed_zero_rejected() {
        let err = ResourceCaps::from_toml_str(r#"memory_max_mb = 0"#).unwrap_err();
        assert!(
            matches!(err, ManifestError::CapOutOfRange { field, .. } if field == "memory_max_mb")
        );
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
        let m = ResourceCaps {
            cpu_max_pct: Some(30),
            memory_max_mb: Some(256),
            fd_max: None,
        };
        let o = ResourceCaps {
            cpu_max_pct: Some(50),
            memory_max_mb: Some(512),
            fd_max: None,
        };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(30));
        assert_eq!(r.memory_max_mb, Some(256));
    }

    #[test]
    fn resolve_caps_operator_wins_when_tighter() {
        let m = ResourceCaps {
            cpu_max_pct: Some(80),
            memory_max_mb: None,
            fd_max: Some(128),
        };
        let o = ResourceCaps {
            cpu_max_pct: Some(40),
            memory_max_mb: Some(1024),
            fd_max: Some(64),
        };
        let r = resolve_caps(&m, &o);
        assert_eq!(r.cpu_max_pct, Some(40));
        assert_eq!(r.memory_max_mb, Some(1024));
        assert_eq!(r.fd_max, Some(64));
    }

    #[test]
    fn resolve_caps_none_falls_through() {
        let m = ResourceCaps {
            cpu_max_pct: None,
            memory_max_mb: Some(256),
            fd_max: None,
        };
        let o = ResourceCaps {
            cpu_max_pct: Some(50),
            memory_max_mb: None,
            fd_max: Some(32),
        };
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
        let s =
            class_toml_full().replace("manifest_schema_version = 1", "manifest_schema_version = 0");
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.manifest_schema_version"))
        );
    }

    // Epic 6 §A4 (retro 2026-05-28) — manifest_schema_version range widened to
    // accept v1 (Epic 1b baseline) ∪ v2 (Epic 6 additive sections). The three
    // tests below pin the contract Story 7.5a will consume for the N-1 / N-2
    // ABI Stability Triple policy.

    #[test]
    fn class_section_accepts_v2_schema_version() {
        // v2 is the current kernel-emitted version (post Epic 6 §A4 bump).
        let s =
            class_toml_full().replace("manifest_schema_version = 1", "manifest_schema_version = 2");
        let c = ClassSection::from_toml_str(&s).unwrap();
        assert_eq!(c.manifest_schema_version, 2);
    }

    #[test]
    fn class_section_still_accepts_v1_schema_version() {
        // v1 (Epic 1b baseline) must still load on a v2 kernel — N-1 supported
        // is the Story 7.5a contract floor. `class_toml_full` is authored at
        // v1 so this is the canonical regression guard.
        let c = ClassSection::from_toml_str(&class_toml_full()).unwrap();
        assert_eq!(c.manifest_schema_version, 1);
    }

    #[test]
    fn class_section_rejects_above_max_schema_version() {
        // Anything beyond MAX_SUPPORTED is hard-rejected — the kernel does not
        // gamble on future schemas it has not been compiled against.
        // (Story 13.5d bumped MAX_SUPPORTED 3→4, so the above-max probe is 5.)
        let s =
            class_toml_full().replace("manifest_schema_version = 1", "manifest_schema_version = 5");
        let err = ClassSection::from_toml_str(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("class.manifest_schema_version"))
        );
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
    fn capabilities_required_loom_defaults_false_and_rejects_unknown_fields() {
        let defaults =
            CapabilitiesRequired::from_toml_str(r#"provider.complete = ["anthropic.default"]"#)
                .unwrap();
        assert_eq!(
            defaults.loom,
            LoomCapabilities {
                read: false,
                write: false,
                scan: false,
            }
        );
        let error = CapabilitiesRequired::from_toml_str(
            r#"provider.complete = ["anthropic.default"]
loom.wirte = true"#,
        )
        .unwrap_err();
        assert!(matches!(error, ManifestError::Toml(_)));
    }

    fn loom_scopes(caps: &CapabilitiesRequired) -> Vec<maos_domain::invariants::i1::Scope> {
        capabilities_required_to_scopes(caps)
            .into_iter()
            .filter(|scope| matches!(scope, maos_domain::invariants::i1::Scope::LoomRead | maos_domain::invariants::i1::Scope::LoomWrite | maos_domain::invariants::i1::Scope::LoomScan))
            .collect()
    }

    #[test]
    fn capabilities_required_loom_degrades_before_schema_v4() {
        let caps = CapabilitiesRequired::from_toml_str(
            "provider.complete = [\"anthropic.default\"]\nloom.write = true",
        )
        .unwrap();
        let v3 = ClassSection::from_toml_str(
            &class_toml_full().replace("manifest_schema_version = 1", "manifest_schema_version = 3"),
        ).unwrap();
        let v4 = ClassSection::from_toml_str(
            &class_toml_full().replace("manifest_schema_version = 1", "manifest_schema_version = 4"),
        ).unwrap();
        assert_eq!(loom_scopes(&caps.clone().degrade_for_schema_version(v3.manifest_schema_version)), Vec::new());
        assert_eq!(
            loom_scopes(&caps.degrade_for_schema_version(v4.manifest_schema_version)),
            vec![maos_domain::invariants::i1::Scope::LoomWrite]
        );
    }

    #[test]
    fn capabilities_required_loom_maps_each_field_exactly() {
        use maos_domain::invariants::i1::Scope;
        for (declaration, expected) in [
            ("loom.read = true", vec![Scope::LoomRead]),
            ("loom.write = true", vec![Scope::LoomWrite]),
            ("loom.scan = true", vec![Scope::LoomScan]),
        ] {
            let caps = CapabilitiesRequired::from_toml_str(&format!("provider.complete = [\"anthropic.default\"]\n{declaration}")).unwrap();
            assert_eq!(loom_scopes(&caps), expected);
        }
    }

    #[test]
    fn capabilities_required_loom_false_fields_emit_no_loom_scopes() {
        for declaration in ["loom.read = false", "loom.write = false", "loom.scan = false", "loom.read = false\nloom.write = false\nloom.scan = false"] {
            let caps = CapabilitiesRequired::from_toml_str(&format!("provider.complete = [\"anthropic.default\"]\n{declaration}")).unwrap();
            assert_eq!(loom_scopes(&caps), Vec::new());
        }
    }

    #[test]
    fn capabilities_required_loom_rejects_malformed_fields() {
        for declaration in ["loom.read = \"yes\"", "loom.write = \"yes\"", "loom.scan = \"yes\""] {
            assert!(CapabilitiesRequired::from_toml_str(&format!("provider.complete = [\"anthropic.default\"]\n{declaration}")).is_err());
        }
    }

    #[test]
    fn capabilities_required_rejects_empty_complete() {
        let s = r#"provider.complete = []"#;
        let err = CapabilitiesRequired::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("capabilities.required.provider.complete"))
        );
    }

    #[test]
    fn capabilities_required_rejects_overlong_entry() {
        let long = "a".repeat(200);
        let s = format!(r#"provider.complete = ["{long}"]"#);
        let err = CapabilitiesRequired::from_toml_str(&s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("capabilities.required.provider.complete"))
        );
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("output_shape.required_fields"))
        );
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("validation failed for"))
        );
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
        let shape = OutputShape {
            required_fields: vec!["a".into(), "b".into()],
        };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"b": 1});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::MissingField { name } if name == "a"));
    }

    #[test]
    fn predicate_misses_second_field() {
        let shape = OutputShape {
            required_fields: vec!["a".into(), "b".into()],
        };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"a": 1});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::MissingField { name } if name == "b"));
    }

    #[test]
    fn predicate_rejects_null_field() {
        let shape = OutputShape {
            required_fields: vec!["a".into()],
        };
        let pred = OutputShapePredicate::from(&shape);
        let value = json!({"a": null});
        let err = pred.check(&value).unwrap_err();
        assert!(matches!(err, OutputShapeViolation::NullField { name } if name == "a"));
    }

    #[test]
    fn predicate_accepts_empty_object_for_empty_required_fields() {
        // Empty predicate trivially satisfied.
        let shape = OutputShape {
            required_fields: vec![],
        };
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.context_window_size"))
        );
    }

    #[test]
    fn budget_rejects_zero_time_cap_seconds() {
        let s = "context_window_size = 4096\ntime_cap_seconds = 0";
        let err = Budget::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.time_cap_seconds"))
        );
    }

    #[test]
    fn budget_rejects_excessive_time_cap() {
        let s = "context_window_size = 4096\ntime_cap_seconds = 86401";
        let err = Budget::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("budget.time_cap_seconds"))
        );
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("epistemic_policy.on_confidence_below"))
        );
    }

    #[test]
    fn epistemic_policy_rejects_threshold_below_zero() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = -0.1"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("epistemic_policy.on_confidence_below"))
        );
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("tag must be non-empty"))
        );
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
        let s = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "tests/fixtures/manifest/epistemic_policy/malformed-rejected/default_action.toml",
        ))
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("lower") && msg.contains("upper"))
        );
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
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("lower") && msg.contains("upper"))
        );
    }

    #[test]
    fn predicate_rejects_both_forms_set() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_value_above = { threshold = 0.5 }
on_value_below = { threshold = 0.3 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("multiple predicate forms"))
        );
    }

    #[test]
    fn predicate_rejects_both_confidence_below_and_on_value() {
        let s = r#"[[rules]]
tag = "x"
action = "halt"
on_confidence_below = 0.5
on_value_above = { threshold = 0.8 }"#;
        let err = EpistemicPolicySection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("both on_confidence_below and on_value_*"))
        );
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
        let err = SchedulingSection::from_toml_str("idle_window_ms = 4000000").unwrap_err();
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
        let err = LifecycleSection::from_toml_str(r#"enabled_hooks = ["on_load", "on_missing"]"#)
            .unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("on_missing")),
            "got: {err:?}"
        );
    }

    #[test]
    fn lifecycle_malformed_duplicate_rejected() {
        let err = LifecycleSection::from_toml_str(r#"enabled_hooks = ["on_load", "on_load"]"#)
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

    // ---- McpSection (Story 5.5c, Fix #26) ----

    fn mcp_section_toml_valid() -> &'static str {
        r#"
[[servers]]
name = "my-server"
uri = "http://localhost:8080"
transport = "stdio"
allowed_tools = ["echo", "cat"]
"#
    }

    #[test]
    fn mcp_section_well_formed_parse() {
        let section = McpSection::from_toml_str(mcp_section_toml_valid()).unwrap();
        assert_eq!(section.servers.len(), 1);
        assert_eq!(section.servers[0].name, "my-server");
        assert_eq!(section.servers[0].uri, "http://localhost:8080");
        assert_eq!(section.servers[0].allowed_tools, vec!["echo", "cat"]);
    }

    #[test]
    fn mcp_section_rejects_empty_name() {
        let s = r#"
[[servers]]
name = ""
uri = "http://localhost:8080"
transport = "stdio"
allowed_tools = ["echo"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("name must not be empty")),
            "got: {err:?}"
        );
    }

    #[test]
    fn mcp_section_rejects_duplicate_names() {
        let s = r#"
[[servers]]
name = "dup"
uri = "http://a:8080"
transport = "stdio"
allowed_tools = ["echo"]

[[servers]]
name = "dup"
uri = "http://b:9090"
transport = "sse"
allowed_tools = ["cat"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("duplicate")),
            "got: {err:?}"
        );
    }

    #[test]
    fn mcp_section_rejects_empty_uri() {
        let s = r#"
[[servers]]
name = "srv"
uri = ""
transport = "stdio"
allowed_tools = ["echo"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("uri must not be empty")),
            "got: {err:?}"
        );
    }

    #[test]
    fn mcp_section_rejects_fallback_equals_primary() {
        let s = r#"
[[servers]]
name = "srv"
uri = "http://localhost:8080"
transport = "stdio"
fallback_transport = "stdio"
allowed_tools = ["echo"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("fallback_transport must differ from primary")),
            "got: {err:?}"
        );
    }

    #[test]
    fn mcp_section_rejects_public_vetted_tier() {
        let s = r#"
[[servers]]
name = "srv"
uri = "http://localhost:8080"
transport = "stdio"
server_trust_tier = "public_vetted"
allowed_tools = ["echo"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("public_vetted")),
            "got: {err:?}"
        );
    }

    #[test]
    fn mcp_section_rejects_glob_in_allowed_tools() {
        let s = r#"
[[servers]]
name = "srv"
uri = "http://localhost:8080"
transport = "stdio"
allowed_tools = ["echo", "wild*"]
"#;
        let err = McpSection::from_toml_str(s).unwrap_err();
        assert!(
            matches!(err, ManifestError::Toml(ref msg) if msg.contains("empty-or-glob")),
            "got: {err:?}"
        );
    }
}

// ------------------------------------------------------------------
// Story 5.2: Hot-swap manifest sections
// ------------------------------------------------------------------

/// The `[hot_swap]` manifest section (Story 5.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HotSwapManifestSection {
    pub state_schema_uri: String,
    pub state_schema_version: u32,
}

/// The `[migrates_from]` manifest section (Story 5.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MigratesFromSection {
    pub versions: Vec<String>,
}

/// The `[halt_protocol_compatibility]` manifest section (Story 5.2).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HaltProtocolCompatibilitySection {
    pub version: u32,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHotSwapSection {
    #[serde(default)]
    state_schema_uri: String,
    #[serde(default)]
    state_schema_version: u32,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMigratesFromSection {
    #[serde(default)]
    versions: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHaltProtocolSection {
    #[serde(default)]
    version: u32,
}

impl HotSwapManifestSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawHotSwapSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        if raw.state_schema_version == 0 {
            return Err(ManifestError::Toml(validation_msg(
                "hot_swap.state_schema_version",
                "must be > 0",
            )));
        }
        Ok(Self {
            state_schema_uri: raw.state_schema_uri,
            state_schema_version: raw.state_schema_version,
        })
    }
}

impl MigratesFromSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawMigratesFromSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        for v in &raw.versions {
            if v.is_empty() {
                return Err(ManifestError::Toml(validation_msg(
                    "migrates_from.versions",
                    "must be non-empty",
                )));
            }
        }
        Ok(Self {
            versions: raw.versions,
        })
    }
}

impl HaltProtocolCompatibilitySection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawHaltProtocolSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        if raw.version == 0 {
            return Err(ManifestError::Toml(validation_msg(
                "halt_protocol_compatibility.version",
                "must be > 0",
            )));
        }
        Ok(Self {
            version: raw.version,
        })
    }
}

#[cfg(test)]
mod hot_swap_manifest_tests {
    use super::*;

    #[test]
    fn hot_swap_well_formed() {
        let toml = r#"state_schema_uri = "https://example.com/schema"
state_schema_version = 1"#;
        let section = HotSwapManifestSection::from_toml_str(toml).unwrap();
        assert_eq!(section.state_schema_version, 1);
    }

    #[test]
    fn hot_swap_rejects_zero_schema_version() {
        let toml = r#"state_schema_uri = ""
state_schema_version = 0"#;
        let result = HotSwapManifestSection::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn migrates_from_well_formed() {
        let toml = r#"versions = ["0.3.x", "0.4.0"]"#;
        let section = MigratesFromSection::from_toml_str(toml).unwrap();
        assert_eq!(section.versions.len(), 2);
    }

    #[test]
    fn migrates_from_rejects_empty_version() {
        let toml = r#"versions = [""]"#;
        let result = MigratesFromSection::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn halt_protocol_compat_well_formed() {
        let toml = r#"version = 1"#;
        let section = HaltProtocolCompatibilitySection::from_toml_str(toml).unwrap();
        assert_eq!(section.version, 1);
    }

    #[test]
    fn halt_protocol_compat_rejects_zero() {
        let toml = r#"version = 0"#;
        let result = HaltProtocolCompatibilitySection::from_toml_str(toml);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod supervision_manifest_tests {
    use super::*;

    #[test]
    fn on_crash_default_nack() {
        let section = OnCrashSection::from_toml_str("").unwrap();
        assert_eq!(
            section.action,
            maos_domain::supervision::OnCrashAction::Nack
        );
    }

    #[test]
    fn on_crash_explicit_reassign() {
        let toml = r#"action = "reassign-to-replica""#;
        let section = OnCrashSection::from_toml_str(toml).unwrap();
        assert_eq!(
            section.action,
            maos_domain::supervision::OnCrashAction::ReassignToReplica
        );
    }

    #[test]
    fn on_crash_explicit_escalate() {
        let toml = r#"action = "escalate-to-operator""#;
        let section = OnCrashSection::from_toml_str(toml).unwrap();
        assert_eq!(
            section.action,
            maos_domain::supervision::OnCrashAction::EscalateToOperator
        );
    }

    #[test]
    fn on_crash_rejects_unknown_action() {
        let toml = r#"action = "panic""#;
        let result = OnCrashSection::from_toml_str(toml);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("unknown value 'panic'"), "{msg}");
    }

    #[test]
    fn supervision_defaults() {
        let section = SupervisionSection::from_toml_str("").unwrap();
        assert_eq!(section.heartbeat_interval_ms, 5000);
        assert_eq!(section.progress_threshold_ms, 30000);
        assert_eq!(section.silent_failure_threshold_ms, 30000);
    }

    #[test]
    fn supervision_explicit_values() {
        let toml = r#"heartbeat_interval_ms = 2000
progress_threshold_ms = 10000
silent_failure_threshold_ms = 15000"#;
        let section = SupervisionSection::from_toml_str(toml).unwrap();
        assert_eq!(section.heartbeat_interval_ms, 2000);
        assert_eq!(section.progress_threshold_ms, 10000);
        assert_eq!(section.silent_failure_threshold_ms, 15000);
    }

    #[test]
    fn supervision_rejects_heartbeat_too_low() {
        let toml = r#"heartbeat_interval_ms = 500"#;
        let result = SupervisionSection::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn supervision_rejects_progress_too_high() {
        let toml = r#"progress_threshold_ms = 400000"#;
        let result = SupervisionSection::from_toml_str(toml);
        assert!(result.is_err());
    }

    #[test]
    fn supervision_rejects_silent_failure_too_low() {
        let toml = r#"silent_failure_threshold_ms = 1000"#;
        let result = SupervisionSection::from_toml_str(toml);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod on_revocation_tests {
    use super::*;

    #[test]
    fn on_revocation_default_terminate_immediately() {
        let section = OnRevocationSection::from_toml_str("").unwrap();
        assert_eq!(
            section.action,
            maos_domain::revocation::RevocationAction::TerminateImmediately
        );
    }

    #[test]
    fn on_revocation_drain_then_terminate() {
        let toml = r#"action = "drain-then-terminate""#;
        let section = OnRevocationSection::from_toml_str(toml).unwrap();
        assert_eq!(
            section.action,
            maos_domain::revocation::RevocationAction::DrainThenTerminate
        );
    }

    #[test]
    fn on_revocation_quarantine() {
        let toml = r#"action = "quarantine""#;
        let section = OnRevocationSection::from_toml_str(toml).unwrap();
        assert_eq!(
            section.action,
            maos_domain::revocation::RevocationAction::Quarantine
        );
    }

    #[test]
    fn on_revocation_rejects_unknown_policy() {
        let toml = r#"action = "unknown-policy""#;
        let result = OnRevocationSection::from_toml_str(toml);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("unknown value 'unknown-policy'"),
            "err={}",
            err
        );
    }

    // ---- ProvidersSection (Story 5.5b, AC2) ----

    #[test]
    fn providers_well_formed_anthropic_no_fallback() {
        let toml = r#"
[primary]
id = "anthropic"
"#;
        let section = ProvidersSection::from_toml_str(toml).unwrap();
        assert_eq!(section.primary.id, "anthropic");
        assert!(section.fallback.is_empty());
    }

    #[test]
    fn providers_well_formed_openai_with_anthropic_fallback() {
        let toml = r#"
[primary]
id = "openai"

[[fallback]]
id = "anthropic"
"#;
        let section = ProvidersSection::from_toml_str(toml).unwrap();
        assert_eq!(section.primary.id, "openai");
        assert_eq!(section.fallback.len(), 1);
        assert_eq!(section.fallback[0].id, "anthropic");
    }

    #[test]
    fn providers_well_formed_ollama_air_gapped() {
        let toml = r#"
[primary]
id = "ollama"
endpoint_url = "http://localhost:11434"
"#;
        let section = ProvidersSection::from_toml_str(toml).unwrap();
        assert_eq!(section.primary.id, "ollama");
        assert_eq!(
            section.primary.endpoint_url.as_deref(),
            Some("http://localhost:11434")
        );
    }

    #[test]
    fn providers_rejects_unsupported_id() {
        let toml = r#"
[primary]
id = "kimi"
"#;
        let err = ProvidersSection::from_toml_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("kimi") && msg.contains("unsupported"),
            "msg={}",
            msg
        );
    }

    #[test]
    fn providers_rejects_empty_endpoint_url() {
        let toml = r#"
[primary]
id = "anthropic"
endpoint_url = ""
"#;
        let err = ProvidersSection::from_toml_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("endpoint_url must not be empty"),
            "msg={}",
            msg
        );
    }

    #[test]
    fn providers_rejects_bad_pin() {
        let toml = r#"
[primary]
id = "anthropic"
provider_endpoint_pin = "not-hex"
"#;
        let err = ProvidersSection::from_toml_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("64-char hex SHA-256"), "msg={}", msg);
    }

    #[test]
    fn providers_accepts_valid_pin() {
        let toml = format!(
            r#"
[primary]
id = "anthropic"
provider_endpoint_pin = "{}"
"#,
            "a".repeat(64)
        );
        let section = ProvidersSection::from_toml_str(&toml).unwrap();
        assert_eq!(
            section.primary.provider_endpoint_pin.as_deref(),
            Some("a".repeat(64).as_str())
        );
    }

    #[test]
    fn providers_fallback_validated_independently() {
        let toml = r#"
[primary]
id = "openai"

[[fallback]]
id = "kimi"
"#;
        let err = ProvidersSection::from_toml_str(toml).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("fallback[0]") && msg.contains("kimi"),
            "msg={}",
            msg
        );
    }
}

// ────────────────────────────────────────────────────────────────────
// Story 6.2 AC5 — [cli_wrapper] manifest section per ADR-021 + architecture §6.7
// ────────────────────────────────────────────────────────────────────

/// `[cli_wrapper]` manifest section. PRESENT means this Spirit is a
/// CliWrapperSpirit; ABSENT means it is a native Rust Spirit using the
/// Spirit ABI directly. The two modes are mutually exclusive at admission
/// (`CliWrapperAdmissionError::EManifestSchemaConflict`).
#[maos_attrs::i9_exempt(
    reason = "manifest data; parsed-then-dropped at admission, no kernel persistence"
)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CliWrapperConfig {
    /// Path to the CLI binary (e.g., `claude`, `opencode`, `gemini-cli`,
    /// `kimi-cli`). Resolved at admission via `which` against the operator's
    /// PATH OR an explicit absolute path. Bare-name resolution is logged with
    /// the resolved absolute path for audit trail (FR52 provenance).
    pub command: String,
    /// Optional argv prefix prepended to every invocation
    /// (e.g., `["code"]` for `claude code`). Empty by default.
    #[serde(default)]
    pub argv_prefix: Vec<String>,
    /// Declared output shape version — semver string. Kernel asserts at
    /// admission; observed shape divergence fires `EOutputShapeAdapterMismatch`.
    pub output_shape_version: String,
    /// Skill bundle: persona + `maos-bridge`. Validated against the Spirit
    /// registry (resolves to `cli-wrapper-template:<cli-name>:<shape-version>`
    /// per architecture §6.7).
    #[serde(default)]
    pub skill_bundle: Vec<String>,
    /// Recovery policy on subprocess death.
    #[serde(default)]
    pub recovery_policy: CliWrapperRecoveryPolicy,
    /// Posture for the subprocess: stdio shape, control-channel mechanism,
    /// shutdown signal.
    pub posture: CliWrapperPosture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum CliWrapperRecoveryPolicy {
    /// Respawn the subprocess with the prior context handed over
    /// (state-transfer across restart). Default for stateful wrappers.
    #[default]
    RespawnWithContext,
    /// Respawn fresh — new conversation, no context transfer. For stateless CLIs.
    RespawnFresh,
    /// Do NOT respawn. Escalate to the supervisor; emit `SpiritDied` per §6.7.
    Escalate,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CliWrapperPosture {
    /// On-wire stdio shape — must round-trip through the registered
    /// output-shape adapter.
    pub stdio_shape: CliWrapperStdioShape,
    /// Control-channel mechanism — how MAOS sends `pause` / `resume` /
    /// `unload` etc.
    pub control_channel: CliWrapperControlChannel,
    /// Signal sent on `unload` lifecycle hook (default SIGTERM; `SIGINT`
    /// for CLIs that handle SIGINT as graceful shutdown).
    #[serde(default)]
    pub shutdown_signal: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum CliWrapperStdioShape {
    NdjsonOverStdio,
    JsonRpcOverStdio,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum CliWrapperControlChannel {
    /// Linux/macOS signals only.
    Signals,
    /// For platforms where signals are inadequate.
    NamedPipe,
    /// In-band stdin control messages.
    StdinCommands,
}

impl CliWrapperConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let cfg: Self = toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validate manifest-level constraints not catchable by serde alone.
    fn validate(&self) -> Result<(), ManifestError> {
        if self.command.trim().is_empty() {
            return Err(ManifestError::Toml(
                "cli_wrapper.command must be non-empty".into(),
            ));
        }
        if let Some(ref sig) = self.posture.shutdown_signal {
            const VALID_SIGNALS: &[&str] = &[
                "SIGTERM", "SIGINT", "SIGKILL", "SIGHUP", "SIGQUIT", "SIGUSR1", "SIGUSR2",
            ];
            if !VALID_SIGNALS.contains(&sig.as_str()) {
                return Err(ManifestError::Toml(format!(
                    "cli_wrapper.posture.shutdown_signal must be one of {VALID_SIGNALS:?}, got \"{sig}\""
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod cli_wrapper_tests {
    use super::*;

    #[test]
    fn cli_wrapper_minimal_round_trip() {
        let toml = r#"
command = "echo"
output_shape_version = "1.0.0"

[posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
"#;
        let cfg = CliWrapperConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.command, "echo");
        assert_eq!(cfg.output_shape_version, "1.0.0");
        assert!(cfg.argv_prefix.is_empty());
        assert_eq!(
            cfg.recovery_policy,
            CliWrapperRecoveryPolicy::RespawnWithContext
        );
        assert_eq!(
            cfg.posture.stdio_shape,
            CliWrapperStdioShape::NdjsonOverStdio
        );
        assert_eq!(
            cfg.posture.control_channel,
            CliWrapperControlChannel::Signals
        );
        assert_eq!(cfg.posture.shutdown_signal, None);
    }

    #[test]
    fn cli_wrapper_full_with_argv_prefix_and_recovery() {
        let toml = r#"
command = "/usr/local/bin/claude"
argv_prefix = ["code"]
output_shape_version = "1.2.3"
skill_bundle = ["orchestrator-bmad", "maos-bridge"]
recovery_policy = "respawn_fresh"

[posture]
stdio_shape = "json_rpc_over_stdio"
control_channel = "stdin_commands"
shutdown_signal = "SIGINT"
"#;
        let cfg = CliWrapperConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.argv_prefix, vec!["code"]);
        assert_eq!(cfg.recovery_policy, CliWrapperRecoveryPolicy::RespawnFresh);
        assert_eq!(
            cfg.posture.stdio_shape,
            CliWrapperStdioShape::JsonRpcOverStdio
        );
        assert_eq!(cfg.posture.shutdown_signal.as_deref(), Some("SIGINT"));
    }

    #[test]
    fn cli_wrapper_rejects_empty_command() {
        let toml = r#"
command = ""
output_shape_version = "1.0.0"

[posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
"#;
        let err = CliWrapperConfig::from_toml_str(toml).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("command must be non-empty"),
            "expected validation error, got: {msg}"
        );
    }

    #[test]
    fn cli_wrapper_rejects_invalid_shutdown_signal() {
        let toml = r#"
command = "echo"
output_shape_version = "1.0.0"

[posture]
stdio_shape = "ndjson_over_stdio"
control_channel = "signals"
shutdown_signal = "SIGTEM"
"#;
        let err = CliWrapperConfig::from_toml_str(toml).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("shutdown_signal must be one of"),
            "expected validation error, got: {msg}"
        );
    }

    #[test]
    fn cli_wrapper_accepts_without_shutdown_signal() {
        let toml = r#"
command = "/usr/bin/env"
output_shape_version = "1.0.0"

[posture]
stdio_shape = "raw"
control_channel = "signals"
"#;
        let cfg = CliWrapperConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.command, "/usr/bin/env");
        assert_eq!(cfg.posture.shutdown_signal, None);
    }

    // ---- SchedulesSection (Story 6.4, AC2 Task 1) ----

    #[test]
    fn schedule_well_formed_round_trip() {
        let toml = r#"
[[schedule]]
id = "morning-digest"
cadence_secs = 3600
payload_b64 = ""
rate_limit_per_hour = 60
principal_revocability = true
side_effect_scopes = [
  { MemWrite = { scope = "spirit:butler:digest" } },
  { ProviderInfer = { provider = "anthropic" } },
]
"#;
        let s = SchedulesSection::from_toml_str(toml).unwrap();
        assert_eq!(s.entries.len(), 1);
        let e = &s.entries[0];
        assert_eq!(e.id, "morning-digest");
        assert_eq!(e.cadence_secs, 3600);
        assert!(e.payload_bytes.is_empty());
        assert_eq!(e.rate_limit_per_hour, 60);
        assert!(e.principal_revocability);
        assert_eq!(e.side_effect_scopes.len(), 2);
    }

    #[test]
    fn schedule_deny_unknown_field_rejected() {
        let toml = r#"
[[schedule]]
id = "x"
cadence_secs = 60
unknown_field = "boom"
"#;
        let err = SchedulesSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("unknown_field")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_cadence_out_of_range_rejected() {
        // cadence_secs = 0 < 1 floor
        let toml_zero = r#"
[[schedule]]
id = "x"
cadence_secs = 0
"#;
        let err = SchedulesSection::from_toml_str(toml_zero).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("cadence_secs")),
            "got: {err:?}"
        );
        // cadence_secs = 604801 > 604800 (1 week) ceiling
        let toml_max = r#"
[[schedule]]
id = "x"
cadence_secs = 604801
"#;
        let err = SchedulesSection::from_toml_str(toml_max).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("cadence_secs")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_rate_limit_out_of_range_rejected() {
        let toml_zero = r#"
[[schedule]]
id = "x"
cadence_secs = 60
rate_limit_per_hour = 0
"#;
        let err = SchedulesSection::from_toml_str(toml_zero).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("rate_limit_per_hour")),
            "got: {err:?}"
        );
        let toml_max = r#"
[[schedule]]
id = "x"
cadence_secs = 60
rate_limit_per_hour = 3601
"#;
        let err = SchedulesSection::from_toml_str(toml_max).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("rate_limit_per_hour")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_duplicate_id_rejected() {
        let toml = r#"
[[schedule]]
id = "dup"
cadence_secs = 60

[[schedule]]
id = "dup"
cadence_secs = 120
"#;
        let err = SchedulesSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("duplicate")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_empty_section_default() {
        // No `[[schedule]]` entries — parses to empty SchedulesSection.
        let s = SchedulesSection::from_toml_str("").unwrap();
        assert!(s.entries.is_empty());
        let s = SchedulesSection::default();
        assert!(s.entries.is_empty());
    }

    #[test]
    fn schedule_compliance_claim_ref_hex_invalid_length() {
        let toml = r#"
[[schedule]]
id = "x"
cadence_secs = 60
compliance_claim_ref_hex = "deadbeef"
"#;
        let err = SchedulesSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("compliance_claim_ref_hex")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_side_effect_scopes_round_trip() {
        // Confirms Scope subtypes round-trip through the manifest parser.
        let toml = r#"
[[schedule]]
id = "arxiv"
cadence_secs = 86400
side_effect_scopes = [
  { NetHttps = { domain = "export.arxiv.org" } },
  { MemWrite = { scope = "spirit:researcher:papers" } },
]
"#;
        let s = SchedulesSection::from_toml_str(toml).unwrap();
        let e = &s.entries[0];
        assert_eq!(e.side_effect_scopes.len(), 2);
        match &e.side_effect_scopes[0] {
            maos_domain::invariants::i1::Scope::NetHttps { domain } => {
                assert_eq!(domain, "export.arxiv.org");
            }
            other => panic!("expected NetHttps, got {other:?}"),
        }
        match &e.side_effect_scopes[1] {
            maos_domain::invariants::i1::Scope::MemWrite { scope } => {
                assert_eq!(scope, "spirit:researcher:papers");
            }
            other => panic!("expected MemWrite, got {other:?}"),
        }
    }

    #[test]
    fn schedule_payload_b64_round_trip() {
        let toml = r#"
[[schedule]]
id = "arxiv-watch"
cadence_secs = 60
payload_b64 = "eyJxdWVyeSI6ImNzLkFJIn0="
"#;
        let s = SchedulesSection::from_toml_str(toml).unwrap();
        let e = &s.entries[0];
        assert_eq!(
            std::str::from_utf8(&e.payload_bytes).unwrap(),
            r#"{"query":"cs.AI"}"#
        );
    }

    #[test]
    fn schedule_compliance_claim_ref_hex_valid_with_sha256_prefix() {
        let hex64 = "0".repeat(64);
        let toml = format!(
            r#"
[[schedule]]
id = "x"
cadence_secs = 60
compliance_claim_ref_hex = "sha256:{hex64}"
"#
        );
        let s = SchedulesSection::from_toml_str(&toml).unwrap();
        assert_eq!(s.entries[0].compliance_claim_ref, Some([0u8; 32]));
    }

    #[test]
    fn schedule_id_regex_rejects_bad_chars() {
        let toml = r#"
[[schedule]]
id = "has space"
cadence_secs = 60
"#;
        let err = SchedulesSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("schedule.id")),
            "got: {err:?}"
        );
    }

    #[test]
    fn schedule_two_entries_distinct_ids() {
        let toml = r#"
[[schedule]]
id = "a"
cadence_secs = 1

[[schedule]]
id = "b"
cadence_secs = 2
"#;
        let s = SchedulesSection::from_toml_str(toml).unwrap();
        assert_eq!(s.entries.len(), 2);
        assert_eq!(s.entries[0].id, "a");
        assert_eq!(s.entries[1].id, "b");
    }
}

// ------------------------------------------------------------------
// [[gateway]] section (Story 6.5 / FR54 / ADR-029)
// ------------------------------------------------------------------

/// Story 6.5 / FR54 / ADR-029 — v0.5 gateway type enumeration. `#[non_exhaustive]`
/// so future gateway implementors can register without an ABI bump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GatewayType {
    Telegram,
    Slack,
    Discord,
    Signal,
    Email,
    /// In-tree reference fixture; exercises the GatewaySubmodule contract
    /// end-to-end without external network deps. NOT for production use.
    Echo,
}

impl Default for GatewayType {
    fn default() -> Self {
        GatewayType::Echo
    }
}

/// Story 6.5 — which Spirit-trait hook receives gateway inbound messages.
/// v0.5: only `OnFrame` supported (FrameKind::GatewayInbound delivered via
/// existing on_frame dispatch). Future: OnInboundMessage adds a dedicated
/// Spirit-trait hook (requires count_hooks!() bump from 14 → 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OnInboundHook {
    OnFrame,
}

impl Default for OnInboundHook {
    fn default() -> Self {
        OnInboundHook::OnFrame
    }
}

/// Story 6.5 / FR54 / ADR-029 — `[[gateway]]` manifest entry.
///
/// Each entry declares one kernel-hosted gateway sub-module that runs as a
/// long-lived connection holder under the Spirit's principal namespace
/// (FR31). The gateway implementation is SPIRIT-SIDE code that registers
/// with the kernel at admission; the kernel runs the lifecycle dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayEntry {
    /// Unique id within this manifest. `[a-zA-Z0-9_-]{1,64}`.
    pub id: String,
    /// Gateway protocol type — `telegram`, `slack`, `discord`, `signal`, `email`, `echo`.
    pub gateway_type: GatewayType,
    /// Reference to a secret in the secrets manager (address-only; the kernel
    /// does NOT interpret the secret content per ADR-026 §4.0.7). Must match
    /// `^secret:[a-z][a-z0-9_-]*:[A-Za-z0-9_-]{1,256}$`.
    pub auth_secret_ref: String,
    /// External recipient identifiers allowed to send inbound messages.
    /// Empty = "no peers allowed" — gateway accepts nothing from external.
    /// Spirit-side gateway code interprets per type.
    pub inbound_allowlist: Vec<String>,
    /// External recipient identifiers allowed for outbound sends.
    /// Empty = explicit "no outbound sends".
    pub outbound_allowlist: Vec<String>,
    /// Hook invoked on inbound messages. Default: `on_frame`.
    pub on_inbound: OnInboundHook,
    /// Initial reconnect delay in seconds. Range `[1, 3600]`; exponential
    /// backoff caps at 5 min. Default: 5.
    pub reconnect_backoff_secs: u32,
    /// Per-message size cap in bytes. Range `[256, 1_048_576]`. Default: 4096.
    pub max_message_bytes: u32,
}

/// The `[[gateway]]` section — `Vec<GatewayEntry>` with cross-entry id
/// uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GatewaysSection {
    pub entries: Vec<GatewayEntry>,
}

impl GatewaysSection {
    pub fn from_toml_str(s: &str) -> Result<Self, ManifestError> {
        let raw: RawGatewaysSection =
            toml::from_str(s).map_err(|e| ManifestError::Toml(e.to_string()))?;
        raw.validate()
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGatewaysSection {
    #[serde(default, rename = "gateway")]
    entries: Vec<RawGatewayEntry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGatewayEntry {
    id: String,
    #[serde(rename = "type")]
    gateway_type: GatewayType,
    auth_secret_ref: String,
    #[serde(default)]
    inbound_allowlist: Vec<String>,
    #[serde(default)]
    outbound_allowlist: Vec<String>,
    #[serde(default = "default_on_inbound")]
    on_inbound: OnInboundHook,
    #[serde(default = "default_reconnect_backoff_secs")]
    reconnect_backoff_secs: u32,
    #[serde(default = "default_max_message_bytes")]
    max_message_bytes: u32,
}

fn default_on_inbound() -> OnInboundHook {
    OnInboundHook::OnFrame
}
fn default_reconnect_backoff_secs() -> u32 {
    5
}
fn default_max_message_bytes() -> u32 {
    4096
}

impl RawGatewaysSection {
    fn validate(self) -> Result<GatewaysSection, ManifestError> {
        let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut entries = Vec::with_capacity(self.entries.len());
        for raw in self.entries {
            // id shape — non-empty, [a-zA-Z0-9_-]{1,64}
            if raw.id.is_empty() || raw.id.len() > 64 {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.id",
                    &format!("must be 1..=64 chars, got len {}", raw.id.len()),
                )));
            }
            if !raw
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.id",
                    &format!("must match [a-zA-Z0-9_-]+, got '{}'", raw.id),
                )));
            }
            if !seen_ids.insert(raw.id.clone()) {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.id",
                    &format!("duplicate gateway id '{}'", raw.id),
                )));
            }
            // auth_secret_ref — must match secret reference pattern
            // ^secret:[a-z][a-z0-9_-]*:[A-Za-z0-9_-]{1,256}$
            if raw.auth_secret_ref.trim().is_empty() {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.auth_secret_ref",
                    "must be non-empty",
                )));
            }
            if !raw.auth_secret_ref.starts_with("secret:") {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.auth_secret_ref",
                    &format!(
                        "must be a secret reference (prefix 'secret:'), got '{}'",
                        raw.auth_secret_ref
                    ),
                )));
            }
            {
                let rest = &raw.auth_secret_ref[7..]; // skip "secret:"
                let colon_pos = rest.find(':');
                if colon_pos.is_none() || colon_pos.unwrap() == 0 {
                    return Err(ManifestError::Toml(validation_msg(
                        "gateway.auth_secret_ref",
                        &format!(
                            "must match secret:<scheme>:<key>, got '{}'",
                            raw.auth_secret_ref
                        ),
                    )));
                }
                let ci = colon_pos.unwrap();
                let scheme = &rest[..ci];
                if scheme.is_empty()
                    || !scheme.starts_with(|c: char| c.is_ascii_lowercase())
                    || !scheme.chars().all(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'
                    })
                {
                    return Err(ManifestError::Toml(validation_msg(
                        "gateway.auth_secret_ref",
                        &format!("scheme must be [a-z][a-z0-9_-]+, got '{}'", scheme),
                    )));
                }
                let key = &rest[ci + 1..];
                if key.is_empty()
                    || key.len() > 256
                    || !key
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    return Err(ManifestError::Toml(validation_msg(
                        "gateway.auth_secret_ref",
                        &format!("key must be [A-Za-z0-9_-]{{1,256}}, got len {}", key.len()),
                    )));
                }
            }
            // reconnect_backoff_secs — [1, 3600]
            if raw.reconnect_backoff_secs < 1 || raw.reconnect_backoff_secs > 3600 {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.reconnect_backoff_secs",
                    &format!("must be in [1, 3600], got {}", raw.reconnect_backoff_secs),
                )));
            }
            // max_message_bytes — [256, 1_048_576]
            if raw.max_message_bytes < 256 || raw.max_message_bytes > 1_048_576 {
                return Err(ManifestError::Toml(validation_msg(
                    "gateway.max_message_bytes",
                    &format!("must be in [256, 1048576], got {}", raw.max_message_bytes),
                )));
            }
            entries.push(GatewayEntry {
                id: raw.id,
                gateway_type: raw.gateway_type,
                auth_secret_ref: raw.auth_secret_ref,
                inbound_allowlist: raw.inbound_allowlist,
                outbound_allowlist: raw.outbound_allowlist,
                on_inbound: raw.on_inbound,
                reconnect_backoff_secs: raw.reconnect_backoff_secs,
                max_message_bytes: raw.max_message_bytes,
            });
        }
        Ok(GatewaysSection { entries })
    }
}

#[cfg(test)]
mod gateway_tests {
    use super::*;

    // 3.1 — well-formed single entry
    #[test]
    fn gateway_well_formed_telegram() {
        let toml = r#"
[[gateway]]
id = "telegram-bot"
type = "telegram"
auth_secret_ref = "secret:telegram:my-bot-token"
inbound_allowlist = ["chat_id:123456789"]
outbound_allowlist = ["chat_id:123456789"]
on_inbound = "on_frame"
reconnect_backoff_secs = 10
max_message_bytes = 8192
"#;
        let g = GatewaysSection::from_toml_str(toml).unwrap();
        assert_eq!(g.entries.len(), 1);
        let e = &g.entries[0];
        assert_eq!(e.id, "telegram-bot");
        assert_eq!(e.gateway_type, GatewayType::Telegram);
        assert_eq!(e.auth_secret_ref, "secret:telegram:my-bot-token");
        assert_eq!(e.inbound_allowlist, &["chat_id:123456789"]);
        assert_eq!(e.outbound_allowlist, &["chat_id:123456789"]);
        assert_eq!(e.on_inbound, OnInboundHook::OnFrame);
        assert_eq!(e.reconnect_backoff_secs, 10);
        assert_eq!(e.max_message_bytes, 8192);
    }

    // 3.2 — two entries with different ids
    #[test]
    fn gateway_two_entries_distinct_ids() {
        let toml = r#"
[[gateway]]
id = "a"
type = "echo"
auth_secret_ref = "secret:echo:token-a"

[[gateway]]
id = "b"
type = "echo"
auth_secret_ref = "secret:echo:token-b"
"#;
        let g = GatewaysSection::from_toml_str(toml).unwrap();
        assert_eq!(g.entries.len(), 2);
        assert_eq!(g.entries[0].id, "a");
        assert_eq!(g.entries[1].id, "b");
    }

    // 3.3 — duplicate id rejected
    #[test]
    fn gateway_duplicate_id_rejected() {
        let toml = r#"
[[gateway]]
id = "same"
type = "echo"
auth_secret_ref = "secret:a:key"

[[gateway]]
id = "same"
type = "echo"
auth_secret_ref = "secret:b:key"
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("duplicate")),
            "got: {err:?}"
        );
    }

    // 3.4 — id validation: empty, over-64-char, invalid-char
    #[test]
    fn gateway_id_empty_rejected() {
        let toml = r#"
[[gateway]]
id = ""
type = "echo"
auth_secret_ref = "secret:x:key"
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("gateway.id")),
            "got: {err:?}"
        );
    }

    #[test]
    fn gateway_id_regex_rejects_space() {
        let toml = r#"
[[gateway]]
id = "has space"
type = "echo"
auth_secret_ref = "secret:x:key"
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("gateway.id")),
            "got: {err:?}"
        );
    }

    // 3.5 — bare credential rejected (no secret: prefix)
    #[test]
    fn gateway_bare_credential_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "abc-123"
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("secret reference")),
            "got: {err:?}"
        );
    }

    // 3.6 — unknown type rejected
    #[test]
    fn gateway_unknown_type_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "matrix"
auth_secret_ref = "secret:x:key"
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("matrix") || msg.contains("type")),
            "got: {err:?}"
        );
    }

    // 3.7 — reconnect_backoff_secs range
    #[test]
    fn gateway_backoff_zero_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
reconnect_backoff_secs = 0
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("reconnect_backoff_secs")),
            "got: {err:?}"
        );
    }

    #[test]
    fn gateway_backoff_too_high_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
reconnect_backoff_secs = 3601
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("reconnect_backoff_secs")),
            "got: {err:?}"
        );
    }

    // 3.8 — max_message_bytes range
    #[test]
    fn gateway_max_message_bytes_too_low_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
max_message_bytes = 255
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("max_message_bytes")),
            "got: {err:?}"
        );
    }

    #[test]
    fn gateway_max_message_bytes_too_high_rejected() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
max_message_bytes = 1048577
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("max_message_bytes")),
            "got: {err:?}"
        );
    }

    // 3.9 — deny_unknown_fields
    #[test]
    fn gateway_deny_unknown_fields() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
inbound_allowList = ["chat:1"]
"#;
        let err = GatewaysSection::from_toml_str(toml).unwrap_err();
        assert!(
            matches!(&err, ManifestError::Toml(ref msg) if msg.contains("unknown field")),
            "got: {err:?}"
        );
    }

    // 3.10 — empty section parses to default
    #[test]
    fn gateway_empty_section_defaults() {
        let toml = "";
        let g = GatewaysSection::from_toml_str(toml).unwrap();
        assert!(g.entries.is_empty());
    }

    // 3.11 — empty inbound_allowlist allowed
    #[test]
    fn gateway_empty_allowlist_parses() {
        let toml = r#"
[[gateway]]
id = "x"
type = "echo"
auth_secret_ref = "secret:x:key"
inbound_allowlist = []
"#;
        let g = GatewaysSection::from_toml_str(toml).unwrap();
        assert!(g.entries[0].inbound_allowlist.is_empty());
        assert!(g.entries[0].outbound_allowlist.is_empty());
    }

    // 3.12 — defaults round-trip
    #[test]
    fn gateway_well_formed_defaults() {
        let toml = r#"
[[gateway]]
id = "my-gw"
type = "echo"
auth_secret_ref = "secret:echo:token"
"#;
        let g = GatewaysSection::from_toml_str(toml).unwrap();
        let e = &g.entries[0];
        assert_eq!(e.id, "my-gw");
        assert_eq!(e.gateway_type, GatewayType::Echo);
        assert_eq!(e.on_inbound, OnInboundHook::OnFrame);
        assert_eq!(e.reconnect_backoff_secs, 5);
        assert_eq!(e.max_message_bytes, 4096);
        assert!(e.inbound_allowlist.is_empty());
        assert!(e.outbound_allowlist.is_empty());
    }
}

#[cfg(test)]
mod model_provenance_tests {
    use super::*;

    const VALID: &str = r#"
covered_model_id = "anthropic.claude-opus-4-8"
training_data_lineage = ["lineage.public-web.cc-2024", "lineage.licensed.books-corpus"]
last_eval_timestamp = "2026-06-01T00:00:00Z"
"#;

    #[test]
    fn valid_section_parses_and_derives_unix_secs() {
        let s = ModelProvenanceSection::from_toml_str(VALID).expect("valid");
        assert_eq!(s.covered_model_id, "anthropic.claude-opus-4-8");
        assert_eq!(s.training_data_lineage.len(), 2);
        // 2026-06-01T00:00:00Z = 1_780_272_000 unix secs (verified below by inverse).
        assert_eq!(s.last_eval_unix_secs, 1_780_272_000);
    }

    #[test]
    fn free_text_lineage_is_rejected_not_admitted() {
        // D5: pasted prose / PII cannot satisfy the reverse-DNS grammar.
        let toml = r#"
covered_model_id = "m"
training_data_lineage = ["Trained on John Doe's emails, SSN 123-45-6789"]
last_eval_timestamp = "2026-06-01T00:00:00Z"
"#;
        let err = ModelProvenanceSection::from_toml_str(toml).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("training_data_lineage") && msg.contains("NOT free-text"),
            "expected free-text rejection, got: {msg}"
        );
    }

    #[test]
    fn empty_lineage_rejected() {
        let toml = r#"
covered_model_id = "m"
training_data_lineage = []
last_eval_timestamp = "2026-06-01T00:00:00Z"
"#;
        assert!(ModelProvenanceSection::from_toml_str(toml).is_err());
    }

    #[test]
    fn unknown_field_rejected_deny_unknown_fields() {
        let toml = r#"
covered_model_id = "m"
training_data_lineage = ["lineage.a.b"]
last_eval_timestamp = "2026-06-01T00:00:00Z"
extra_field = "nope"
"#;
        assert!(ModelProvenanceSection::from_toml_str(toml).is_err());
    }

    #[test]
    fn malformed_timestamp_rejected() {
        for bad in [
            "2026-06-01",                // date only
            "2026-06-01T00:00:00+02:00", // offset not allowed
            "2026-13-01T00:00:00Z",      // month OOR
            "2026-02-30T00:00:00Z",      // day OOR (Feb)
            "2026-06-01T24:00:00Z",      // hour OOR
            "not-a-date",
        ] {
            let toml = format!(
                "covered_model_id = \"m\"\ntraining_data_lineage = [\"lineage.a.b\"]\nlast_eval_timestamp = \"{bad}\"\n"
            );
            assert!(
                ModelProvenanceSection::from_toml_str(&toml).is_err(),
                "expected reject for bad timestamp {bad:?}"
            );
        }
    }

    #[test]
    fn leap_year_feb_29_accepted() {
        // 2028 is a leap year; Feb 29 must parse.
        let toml = "covered_model_id = \"m\"\ntraining_data_lineage = [\"lineage.a.b\"]\nlast_eval_timestamp = \"2028-02-29T12:00:00Z\"\n";
        let s = ModelProvenanceSection::from_toml_str(toml).expect("leap day valid");
        // sanity: 2028-02-29T12:00:00Z is after 2026-06-01.
        assert!(s.last_eval_unix_secs > 1_780_272_000);
    }

    #[test]
    fn absent_section_is_none_optional_on_read_ac11() {
        // AC-11: a v2 manifest with no [model_provenance] stays admissible.
        let manifest = "[class]\nname = \"x\"\nversion = \"0.1.0\"\n[author]\nname = \"a\"\n";
        let got = ModelProvenanceSection::from_manifest_toml(manifest).expect("ok");
        assert!(got.is_none());
    }

    #[test]
    fn present_section_extracted_from_full_manifest() {
        let manifest =
            format!("[class]\nname = \"x\"\nversion = \"0.1.0\"\n[model_provenance]\n{VALID}");
        let got = ModelProvenanceSection::from_manifest_toml(&manifest)
            .expect("ok")
            .expect("present");
        assert_eq!(got.covered_model_id, "anthropic.claude-opus-4-8");
    }

    #[test]
    fn staleness_rejects_old_eval() {
        use maos_domain::provenance::ProvenanceError;
        let s = ModelProvenanceSection::from_toml_str(VALID).unwrap();
        // now = last_eval + 100 days; window = 30 days → stale.
        let now = s.last_eval_unix_secs + 100 * 86_400;
        let err = s.validate_staleness(now, 30 * 86_400).unwrap_err();
        assert!(matches!(err, ProvenanceError::EModelProvenanceStale { .. }));
        // fresh within window → ok.
        assert!(s
            .validate_staleness(s.last_eval_unix_secs + 10 * 86_400, 30 * 86_400)
            .is_ok());
    }

    #[test]
    fn canonical_content_bytes_are_deterministic_and_field_sensitive() {
        let a = ModelProvenanceSection::from_toml_str(VALID).unwrap();
        let b = ModelProvenanceSection::from_toml_str(VALID).unwrap();
        assert_eq!(a.canonical_content_bytes(), b.canonical_content_bytes());
        let other = ModelProvenanceSection::from_toml_str(
            "covered_model_id = \"different.model\"\ntraining_data_lineage = [\"lineage.a.b\"]\nlast_eval_timestamp = \"2026-06-01T00:00:00Z\"\n",
        )
        .unwrap();
        assert_ne!(a.canonical_content_bytes(), other.canonical_content_bytes());
    }
}
