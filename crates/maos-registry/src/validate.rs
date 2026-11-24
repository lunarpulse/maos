//! Story 22-7 — `maos-spirit validate`: what would the Host refuse about this package?
//!
//! A pure library module: it reads no environment, opens no socket, holds no signing key and
//! writes nothing. The caller hands it a [`SignedPackage`] (the CLI signs the in-memory package
//! with an ephemeral key) and, optionally, the operator's [`RegistryClientConfig`].
//!
//! Three independent checks, one finding per failed check:
//!
//! 1. `manifest`: the **boot gate's** manifest checks. `maos serve` runs
//!    `maos_bin::admission::gate_manifest` over every installed Spirit with a class predicate
//!    that knows no compiled-in class (`installed_spirits.rs`: "Registry installs never
//!    introduce compiled-in first-party classes"). `gate_manifest` lives in `maos-bin`, which
//!    depends on `maos-spirit-cli`, so this module re-states its section order over the same
//!    `maos_manifest` parsers. The refusal codes are `AdmissionRefusal::code()`'s. The
//!    `validate_parity_22_7` test in `maos-bin` holds the two to one verdict on a corpus.
//! 2. `admission`: the **install gate**, [`admit_spirit`] itself, under the operator policy.
//! 3. `component`: the artifact is a WebAssembly *component* shaped as the runner needs it.
//!
//! Not mirrored, because they need a running Host: the provider-catalog pin checks, the
//! model-provenance policy, the registry yank list, and the runner's own compile at launch.

use maos_domain::invariants::i9::SandboxTier as ManifestTier;
use maos_domain::ports::registry::{SignedPackage, TrustTier};
use maos_manifest::{
    Budget, CapabilitiesRequired, ClassSection, ConsentSection, EpistemicPolicySection,
    ExplanationShapeSection, HaltProtocolCompatibilitySection, HotSwapManifestSection,
    IntentPromotionSetSection, LifecycleSection, MigratesFromSection, OnCrashSection,
    OnRevocationSection, OutputShape, PostureSection, ProvidersSection, ResourceCaps,
    SandboxConfig, SchedulesSection, SchedulingSection, SkillsSection, SupervisionSection,
    SwapInvariantsSection,
};
use maos_spirit_abi::compliance::SandboxTier;
use serde::Serialize;

use crate::admission::{admit_spirit, AdmissionConfig, AdmissionError};
use crate::client::RegistryClientConfig;

/// One failed check: which check, the stable code the Host would print, and the detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub check: &'static str,
    pub code: &'static str,
    pub detail: String,
}

/// The report `maos-spirit validate` prints. Empty `findings` means the package would pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationReport {
    pub spirit_id: String,
    pub version: String,
    pub form: String,
    pub effective_tier: Option<&'static str>,
    pub sandbox_tier_floor: Option<&'static str>,
    pub findings: Vec<Finding>,
}

impl ValidationReport {
    pub fn admissible(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Runs the three checks. `cfg` is the operator's registry policy; `None` means the defaults.
pub fn validate(pkg: &SignedPackage, cfg: Option<&RegistryClientConfig>) -> ValidationReport {
    let mut report = ValidationReport {
        spirit_id: String::new(),
        version: String::new(),
        form: String::new(),
        effective_tier: None,
        sandbox_tier_floor: None,
        findings: Vec::new(),
    };
    let mut wasm = false;
    match boot_gate(&pkg.manifest_toml) {
        Ok(class) => {
            wasm = class.forms == ["wasm-component"];
            report.spirit_id = class.name;
            report.version = class.version;
            report.form = class.forms.join(",");
        }
        Err(finding) => report.findings.push(finding),
    }
    let cfg = cfg.cloned().unwrap_or_default();
    let policy = AdmissionConfig {
        tier_floor: cfg.tier_floor,
        // The origin tier a server reports; an offline dry run has none, as `install` defaults it.
        registry_origin_tier: TrustTier::Local,
        t3_for_public_untrusted: cfg.t3_for_public_untrusted,
        allow_unsigned_local: cfg.allow_unsigned_local,
        org_signing_pubkey: cfg.org_signing_pubkey,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };
    match admit_spirit(pkg, &policy) {
        Ok(decision) => {
            report.effective_tier = Some(tier_name(decision.effective_tier));
            report.sandbox_tier_floor = Some(sandbox_name(decision.sandbox_tier_floor));
        }
        Err(error) => report.findings.push(Finding {
            check: "admission",
            code: admission_code(&error),
            detail: error.to_string(),
        }),
    }
    if wasm {
        if let Err(finding) = component_shape(&pkg.artifact_bytes) {
            report.findings.push(finding);
        }
    }
    report
}

/// The stable machine code `maos-spirit validate` reports for an install-gate refusal. Exhaustive,
/// so a new refusal cannot ship without a code. It lives here, not on `AdmissionError`, because
/// `admission.rs` is a pinned FKCS admission file (`xtask/fkcs-baseline.toml`).
pub fn admission_code(error: &AdmissionError) -> &'static str {
    match error {
        AdmissionError::PublicVettedDeferred => "public_vetted_deferred",
        AdmissionError::ManifestTrustTierInvalid(_) => "manifest_trust_tier_invalid",
        AdmissionError::ClassSectionInvalid(_) => "class_section_invalid",
        AdmissionError::FirstPartyFormFromRegistry => "first_party_form_from_registry",
        AdmissionError::OrgSignatureInvalid => "org_signature_invalid",
        AdmissionError::PublisherSignatureInvalid => "publisher_signature_invalid",
        AdmissionError::ComplianceContextDrift { .. } => "compliance_context_drift",
        AdmissionError::UnsignedLocalRejected => "unsigned_local_rejected",
        AdmissionError::ModelProvenance(_) => "model_provenance_refused",
        AdmissionError::ModelProvenanceMalformed(_) => "model_provenance_malformed",
        AdmissionError::OffFrozenSurface { .. } => "off_frozen_surface",
        AdmissionError::VettingAttestationRejected(_) => "vetting_attestation_rejected",
    }
}

fn tier_name(tier: TrustTier) -> &'static str {
    match tier {
        TrustTier::Local => "local",
        TrustTier::OrgInternal => "org_internal",
        TrustTier::PublicUntrusted => "public_untrusted",
        TrustTier::PublicVetted => "public_vetted",
    }
}

fn sandbox_name(tier: SandboxTier) -> &'static str {
    match tier {
        SandboxTier::T0 => "T0",
        SandboxTier::T1 => "T1",
        SandboxTier::T2 => "T2",
        SandboxTier::T3 => "T3",
        SandboxTier::T4 => "T4",
    }
}

fn manifest_finding(code: &'static str, detail: impl Into<String>) -> Finding {
    Finding {
        check: "manifest",
        code,
        detail: detail.into(),
    }
}

fn section_finding(section: &str, detail: impl std::fmt::Display) -> Finding {
    manifest_finding(
        "manifest_section_invalid",
        format!("[{section}] section: {detail}"),
    )
}

/// The section VALUE serialized as `gate_manifest`'s `section` closure does: a value that
/// cannot be a TOML document reads as absent.
fn section_text(root: &toml::Value, name: &str) -> Option<String> {
    root.get(name).and_then(|value| toml::to_string(value).ok())
}

fn required(root: &toml::Value, name: &'static str) -> Result<String, Finding> {
    section_text(root, name).ok_or_else(|| section_finding(name, "section is required and absent"))
}

/// `gate_manifest`'s `optional_section`: absent is fine, present and malformed is refused.
fn optional<T, E: std::fmt::Display>(
    root: &toml::Value,
    name: &'static str,
    parse: fn(&str) -> Result<T, E>,
) -> Result<Option<T>, Finding> {
    root.get(name)
        .map(|value| {
            let text = toml::to_string(value).map_err(|error| error.to_string())?;
            parse(&text).map_err(|error| error.to_string())
        })
        .transpose()
        .map_err(|detail| section_finding(name, detail))
}

/// Gates 2-7 of `gate_manifest`, in its order, for a registry-installed Spirit.
fn boot_gate(manifest: &[u8]) -> Result<ClassSection, Finding> {
    let text = std::str::from_utf8(manifest).map_err(|error| {
        manifest_finding(
            "manifest_parse_failed",
            format!("manifest is not valid TOML: {error}"),
        )
    })?;
    let root: toml::Value = toml::from_str(text).map_err(|error| {
        manifest_finding(
            "manifest_parse_failed",
            format!("manifest is not valid TOML: {error}"),
        )
    })?;
    if root.get("cli_wrapper").is_some() {
        return Err(if root.get("class").is_some() {
            manifest_finding(
                "manifest_schema_conflict",
                "manifest declares both [cli_wrapper] and [class]",
            )
        } else {
            manifest_finding(
                "cli_wrapper_unsupported",
                "[cli_wrapper] manifests declare no Spirit class",
            )
        });
    }
    let class_toml = root
        .get("class")
        .ok_or_else(|| {
            manifest_finding(
                "class_section_invalid",
                "[class] section: section is required and absent",
            )
        })
        .and_then(|class| {
            toml::to_string(class).map_err(|error| {
                manifest_finding("class_section_invalid", format!("[class] section: {error}"))
            })
        })?;
    let class = ClassSection::from_toml_str(&class_toml).map_err(|error| {
        manifest_finding("class_section_invalid", format!("[class] section: {error}"))
    })?;
    let wasm = class.forms == ["wasm-component"];
    if !wasm {
        return Err(manifest_finding(
            "unknown_spirit_class",
            format!(
                "'{}' declares forms {:?}: a registry install admits `wasm-component` Spirits only, never a compiled-in class",
                class.name, class.forms
            ),
        ));
    }

    let sandbox = SandboxConfig::from_toml_str(&required(&root, "sandbox")?)
        .map_err(|error| section_finding("sandbox", error))?;
    if sandbox.tier != ManifestTier::T2 {
        return Err(manifest_finding(
            "wasm_component_requires_t2",
            format!(
                "wasm_component_requires_t2: wasm-component requires sandbox tier T2, got {}",
                sandbox.tier
            ),
        ));
    }
    ResourceCaps::from_toml_str(&required(&root, "resources")?)
        .map_err(|error| section_finding("resources", error))?;
    let caps = match root.get("capabilities").and_then(|c| c.get("required")) {
        Some(value) => {
            let text = toml::to_string(value)
                .map_err(|error| section_finding("capabilities.required", error))?;
            Some(
                CapabilitiesRequired::from_toml_str(&text)
                    .map_err(|error| section_finding("capabilities.required", error))?,
            )
        }
        None => None,
    };
    OutputShape::from_toml_str(&required(&root, "output_shape")?)
        .map_err(|error| section_finding("output_shape", error))?;
    PostureSection::from_toml_str(&required(&root, "posture")?)
        .map_err(|error| section_finding("posture", error))?;

    if let Some(text) = section_text(&root, "epistemic_policy") {
        EpistemicPolicySection::from_toml_str(&text)
            .map_err(|error| section_finding("epistemic_policy", error))?;
    }
    if let Some(text) = section_text(&root, "scheduling") {
        SchedulingSection::from_toml_str(&text)
            .map_err(|error| section_finding("scheduling", error))?;
    }
    let budget = section_text(&root, "budget")
        .map(|text| Budget::from_toml_str(&text).map_err(|error| section_finding("budget", error)))
        .transpose()?;
    let lifecycle = section_text(&root, "lifecycle");
    if let Some(text) = &lifecycle {
        LifecycleSection::from_toml_str(text)
            .map_err(|error| section_finding("lifecycle", error))?;
    }
    // D-21-9-02 A — the version-governed `[lifecycle.telemetry]` declaration.
    maos_manifest::validate_telemetry_subscription(
        class.manifest_schema_version,
        lifecycle.as_deref(),
        budget.as_ref(),
    )
    .map_err(|error| section_finding("lifecycle", error))?;
    optional(&root, "hot_swap", HotSwapManifestSection::from_toml_str)?;
    optional(&root, "migrates_from", MigratesFromSection::from_toml_str)?;
    optional(
        &root,
        "halt_protocol_compatibility",
        HaltProtocolCompatibilitySection::from_toml_str,
    )?;
    optional(&root, "on_crash", OnCrashSection::from_toml_str)?;
    optional(&root, "on_revocation", OnRevocationSection::from_toml_str)?;
    optional(&root, "supervision", SupervisionSection::from_toml_str)?;
    if let Some(providers) = optional(&root, "providers", ProvidersSection::from_toml_str)? {
        let complete = caps
            .as_ref()
            .map(|c| c.provider.complete.as_slice())
            .unwrap_or(&[]);
        for config in std::iter::once(&providers.primary).chain(&providers.fallback) {
            let prefix = format!("{}.", config.id);
            if !complete
                .iter()
                .any(|d| *d == config.id || d.starts_with(&prefix))
            {
                return Err(manifest_finding(
                    "provider_unavailable",
                    format!(
                        "provider_unavailable: '{}' is undeclared, unregistered, or its model assertion differs",
                        config.id
                    ),
                ));
            }
        }
        if !providers.fallback.is_empty() {
            return Err(manifest_finding(
                "fallback_unsupported",
                "fallback_unsupported: a primary token cannot authorise fallback providers",
            ));
        }
    }
    if class.manifest_schema_version >= maos_manifest::FR8_DECLARATIONS_SINCE_SCHEMA {
        optional(&root, "skills", SkillsSection::from_toml_str)?;
        optional(
            &root,
            "explanation_shape",
            ExplanationShapeSection::from_toml_str,
        )?;
        optional(
            &root,
            "intent_promotion_set",
            IntentPromotionSetSection::from_toml_str,
        )?;
        optional(
            &root,
            "swap_invariants",
            SwapInvariantsSection::from_toml_str,
        )?;
    }
    optional(&root, "consent", ConsentSection::from_toml_str)?;
    if let Some(value) = root.get("schedule") {
        let mut wrapper = toml::value::Table::new();
        wrapper.insert("schedule".into(), value.clone());
        let text = toml::to_string(&wrapper).map_err(|error| section_finding("schedule", error))?;
        SchedulesSection::from_toml_str(&text).map_err(|error| match error {
            maos_manifest::ManifestError::ScheduleEntryLimit { .. } => manifest_finding(
                "schedule_entry_limit",
                format!("schedule_entry_limit: {error}"),
            ),
            maos_manifest::ManifestError::ScheduleDecodedBytesLimit { .. } => manifest_finding(
                "schedule_decoded_bytes_limit",
                format!("schedule_decoded_bytes_limit: {error}"),
            ),
            other => section_finding("schedule", other),
        })?;
    }
    Ok(class)
}

fn component_finding(code: &'static str, detail: impl Into<String>) -> Finding {
    Finding {
        check: "component",
        code,
        detail: detail.into(),
    }
}

/// The exports the `maos:spirit` world requires of every guest.
const REQUIRED_EXPORTS: [&str; 3] = ["handle-frame", "on-start", "on-shutdown"];

/// Whether the runner can bind this import: a WASI interface (`wasmtime_wasi::p2` is linked
/// whole) or a `maos:spirit` host interface at major 2 (`frames` is the legacy world's, 0 or 1).
fn import_is_provided(name: &str) -> bool {
    if name.starts_with("wasi:") {
        return true;
    }
    let Some(rest) = name.strip_prefix("maos:spirit/") else {
        return false;
    };
    let Some((interface, version)) = rest.split_once('@') else {
        return false;
    };
    let major = version
        .split('.')
        .next()
        .and_then(|m| m.parse::<u64>().ok());
    match interface {
        "recall" | "context" | "skills" => major == Some(2),
        "frames" => matches!(major, Some(0..=2)),
        _ => false,
    }
}

/// The artifact is a component, exports the three world functions, optionally the 2.x
/// `lifecycle` interface, and imports only what the runner links.
fn component_shape(artifact: &[u8]) -> Result<(), Finding> {
    use wasmparser::{Encoding, Parser, Payload};
    let (mut imports, mut exports) = (Vec::new(), Vec::new());
    let (mut depth, mut seen_version) = (0usize, false);
    for payload in Parser::new(0).parse_all(artifact) {
        let malformed = |error: wasmparser::BinaryReaderError| {
            component_finding(
                "component_malformed",
                format!("not a parsable WebAssembly binary: {error}"),
            )
        };
        match payload.map_err(malformed)? {
            Payload::Version { encoding, .. } if !seen_version => {
                seen_version = true;
                if encoding != Encoding::Component {
                    return Err(component_finding(
                        "component_not_a_component",
                        "the artifact is a core WebAssembly module; build a component (`npm run componentize`)",
                    ));
                }
            }
            Payload::ModuleSection { .. } | Payload::ComponentSection { .. } => depth += 1,
            Payload::End(_) => depth = depth.saturating_sub(1),
            Payload::ComponentImportSection(reader) if depth == 0 => {
                for import in reader {
                    let import = import.map_err(malformed)?;
                    // `use frames.{iac-frame, halt}` imports types by bare name; nothing to link.
                    if !matches!(import.ty, wasmparser::ComponentTypeRef::Type(_)) {
                        imports.push(import.name.name.to_owned());
                    }
                }
            }
            Payload::ComponentExportSection(reader) if depth == 0 => {
                for export in reader {
                    exports.push(export.map_err(malformed)?.name.name.to_owned());
                }
            }
            _ => {}
        }
    }
    if !seen_version {
        return Err(component_finding(
            "component_malformed",
            "the artifact is empty",
        ));
    }
    let missing: Vec<&str> = REQUIRED_EXPORTS
        .into_iter()
        .filter(|name| !exports.iter().any(|export| export == name))
        .collect();
    if !missing.is_empty() {
        return Err(component_finding(
            "component_missing_export",
            format!("the component does not export {missing:?} (world maos:spirit)"),
        ));
    }
    if let Some(bad) = exports.iter().find(|name| {
        name.strip_prefix("maos:spirit/lifecycle@")
            .is_some_and(|version| !version.starts_with("2."))
    }) {
        return Err(component_finding(
            "component_unsupported_export",
            format!("{bad}: the runner binds `maos:spirit/lifecycle` at major 2 only"),
        ));
    }
    let unprovided: Vec<&String> = imports
        .iter()
        .filter(|name| !import_is_provided(name))
        .collect();
    if !unprovided.is_empty() {
        return Err(component_finding(
            "component_unsupported_import",
            format!("imports the runner does not provide: {unprovided:?}"),
        ));
    }
    Ok(())
}
