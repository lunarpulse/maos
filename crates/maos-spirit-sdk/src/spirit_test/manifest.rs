#![forbid(unsafe_code)]

//! Manifest self-check primitive — parses raw TOML bytes through a
//! minimal-manifest-shape and returns a typed report listing parsed
//! sections + violations + edge-case warnings.
//!
//! **Duplication note (Story 2.4 dev record):** The kernel-side section
//! parsers at `crates/maos-kernel-core/src/security/manifest.rs` are the
//! authoritative implementations. This SDK-side re-skin is intentional
//! to preserve the zero-kernel-dep constraint (the SDK is consumed by
//! third-party Spirit crates that must not transitively pull in the
//! kernel). Future Story 7.1 may consolidate by extracting a shared
//! manifest-types sub-crate; tracked in the dev record.

use serde::Deserialize;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManifestSelfCheckReport {
    pub class_name: String,
    pub class_version: String,
    pub forms: Vec<String>,
    pub trust_tier: String,
    pub capabilities_required_count: usize,
    pub posture_default: String,
    pub posture_allowed_max: String,
    pub output_shape_required_fields: Vec<String>,
    pub budget_context_window_size: Option<u32>,
    pub budget_time_cap_seconds: Option<u32>,
    pub resources_cpu_max_pct: Option<u32>,
    pub resources_memory_max_mb: Option<u32>,
    pub sandbox_tier: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestSelfCheckViolation {
    TomlParseError(String),
    MissingRequiredSection(&'static str),
    InvalidValue {
        field: &'static str,
        value: String,
        reason: &'static str,
    },
}

#[derive(Deserialize)]
struct ManifestMinimalShape {
    class: ClassSection,
    capabilities: Option<CapabilitiesSection>,
    posture: PostureSection,
    output_shape: Option<OutputShapeSection>,
    budget: Option<BudgetSection>,
    resources: Option<ResourcesSection>,
    sandbox: SandboxSection,
}
#[derive(Deserialize)]
struct ClassSection {
    name: String,
    version: String,
    forms: Vec<String>,
    trust_tier: String,
}
#[derive(Deserialize)]
struct CapabilitiesSection {
    required: Option<toml::Table>,
}
#[derive(Deserialize)]
struct PostureSection {
    default: String,
    allowed_max: String,
}
#[derive(Deserialize)]
struct OutputShapeSection {
    required_fields: Vec<String>,
}
#[derive(Deserialize)]
struct BudgetSection {
    context_window_size: Option<u32>,
    time_cap_seconds: Option<u32>,
}
#[derive(Deserialize)]
struct ResourcesSection {
    cpu_max_pct: Option<u32>,
    memory_max_mb: Option<u32>,
}
#[derive(Deserialize)]
struct SandboxSection {
    tier: String,
}

pub fn manifest_self_check(
    manifest_toml_bytes: &[u8],
) -> Result<ManifestSelfCheckReport, ManifestSelfCheckViolation> {
    let toml_str = std::str::from_utf8(manifest_toml_bytes)
        .map_err(|e| ManifestSelfCheckViolation::TomlParseError(format!("non-UTF-8: {e}")))?;
    let toml_str = toml_str.strip_prefix("\u{feff}").unwrap_or(toml_str);
    let parsed: ManifestMinimalShape = toml::from_str(toml_str)
        .map_err(|e| ManifestSelfCheckViolation::TomlParseError(e.to_string()))?;

    let mut warnings = Vec::new();
    if parsed.class.forms.is_empty() {
        warnings.push("class.forms is empty — Spirit cannot be loaded in any form".to_string());
    } else {
        for (i, form) in parsed.class.forms.iter().enumerate() {
            if form.is_empty() {
                warnings.push(format!(
                    "class.forms[{i}] is empty — Spirit cannot be loaded in an unnamed form"
                ));
            }
        }
    }
    if let Some(ref os) = parsed.output_shape {
        for (i, f) in os.required_fields.iter().enumerate() {
            if f.is_empty() {
                return Err(ManifestSelfCheckViolation::InvalidValue {
                    field: "output_shape.required_fields",
                    value: format!("<empty at index {i}>"),
                    reason: "field names must not be empty",
                });
            }
            if f.contains(' ') {
                return Err(ManifestSelfCheckViolation::InvalidValue {
                    field: "output_shape.required_fields",
                    value: "<contains whitespace>".to_string(),
                    reason: "field names must not contain whitespace (Story 2.1 AC3)",
                });
            }
        }
    }
    if !matches!(
        parsed.sandbox.tier.as_str(),
        "T0" | "T1" | "T2" | "T3" | "T4"
    ) {
        return Err(ManifestSelfCheckViolation::InvalidValue {
            field: "sandbox.tier",
            value: parsed.sandbox.tier.clone(),
            reason: "tier must be one of T0/T1/T2/T3/T4",
        });
    }

    Ok(ManifestSelfCheckReport {
        class_name: parsed.class.name,
        class_version: parsed.class.version,
        forms: parsed.class.forms,
        trust_tier: parsed.class.trust_tier,
        capabilities_required_count: parsed
            .capabilities
            .and_then(|c| c.required)
            .map(|t| t.len())
            .unwrap_or(0),
        posture_default: parsed.posture.default,
        posture_allowed_max: parsed.posture.allowed_max,
        output_shape_required_fields: parsed
            .output_shape
            .map(|o| o.required_fields)
            .unwrap_or_default(),
        budget_context_window_size: parsed.budget.as_ref().and_then(|b| b.context_window_size),
        budget_time_cap_seconds: parsed.budget.as_ref().and_then(|b| b.time_cap_seconds),
        resources_cpu_max_pct: parsed.resources.as_ref().and_then(|r| r.cpu_max_pct),
        resources_memory_max_mb: parsed.resources.as_ref().and_then(|r| r.memory_max_mb),
        sandbox_tier: parsed.sandbox.tier,
        warnings,
    })
}
