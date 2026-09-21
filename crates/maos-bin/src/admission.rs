#![cfg(feature = "network")]

//! Story 16-6 — the ONE admission path.
//!
//! Before this module, `maos run` carried the `load → admit → start` triple
//! twice over (the standalone arm and the topology loop) and the operator door
//! carried it nowhere: `maosctl` had no `load` verb at all. Adding one by
//! calling `SpiritSchedulerAdapter::load` from the door was never an option —
//! the daemon constructs its scheduler with `security_manager: None`, so the
//! kernel's own internal `admit_spirit` **never runs in the daemon** and a
//! door-side load would have admitted the Spirit not at a lower tier but not
//! at all. Every gate that matters lives in `maos-bin`. So it lives here,
//! once, and all three callers reach it.
//!
//! ## Order
//!
//! `provenance → load → admit → start`, which is the STANDALONE arm's order,
//! not the topology arm's. Topology ran model-provenance *after* admission, so
//! a provenance-refused Spirit had already been admitted at a real pid; it
//! also emitted `emit_vetter_key_event` on grant only, leaving an admission
//! rejection with no audit row at all. Both are resolved by re-pointing it
//! here.
//!
//! Admission runs **after** load and not before, on every path, because
//! capability mediation is keyed by pid and the pid does not exist until
//! `allocate_pid()` inside `load`. Admitting a placeholder pid would make
//! every live port fail closed.
//!
//! ## Rollback
//!
//! Because admission follows the load, a refusal leaves a Spirit loaded in the
//! scheduler and unadmitted. At HEAD `maos run` simply returned the error out
//! of `main` and the process exited, so nobody noticed; over a door the daemon
//! keeps serving and the leak is permanent. Every failure after the load is
//! therefore unwound through `scheduler.unload`, which is only possible at all
//! because this story added the kernel's `(Loaded, Unloaded)` arm. **The
//! partial-admission leak and the missing transition are the same defect seen
//! from two ends.**
//!
//! ## What this module must never touch
//!
//! The manifest-derived policy-scope table. `admission.rs` is enrolled in
//! the `SCANNED_SOURCE_FILES` roster that feeds the negative in
//! `tests/cohort_daemon_smoke_13_5c.rs`, and it is NOT whitelisted the way
//! `enterprise_pdp_runtime.rs` is — so that negative fails on the mere
//! mention of the table's name anywhere in this file, comments included.
//! Seeding or consuming it is `admit_spirit`'s job, not the caller's.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use maos_kernel_core::scheduler::control_block::{AnySpiritObj, SpiritManifestBundle};
use maos_kernel_core::scheduler::scheduler_loop::SpiritSchedulerAdapter;
use maos_kernel_core::security::manifest::{
    Budget, CapabilitiesRequired, ClassSection, EpistemicPolicySection, LifecycleSection,
    OutputShape, Posture, PostureSection, ResourceCaps, SandboxConfig, SchedulingSection,
};
use maos_kernel_core::security::SecurityManagerAdapter;

/// Emit a `FrameKind::GovernanceEvent` with a `VetterKeyPayload` to the
/// Transparency Log. Consolidates the formerly copy-pasted emission blocks
/// (Story 9.3b review — VetterKey emission coverage).
///
/// Called on every admission, rejection, and rotation decision point so the
/// audit trail records the full trust-tier decision history.
pub fn emit_vetter_key_event(
    tl: &maos_kernel_core::iac::TransparencyLogAdapter,
    spirit_id: &str,
    version: &str,
    admitted: bool,
    effective_tier: &str,
    journal_note: &str,
) {
    // Fallback to epoch-zero when the system clock is before UNIX_EPOCH
    // (e.g. pre-epoch embedded / VM clocks). A zero timestamp is
    // preferable to a panic in a production governance-emission path
    // (Story 9.3b patch 2).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos() as u64;
    let gov_payload = maos_domain::governance::GovernanceEventPayload {
        recorded_at_ns: now,
        effective_at_ns: now,
        event: maos_domain::governance::GovernanceEventKind::VetterKey(
            maos_domain::governance::VetterKeyPayload {
                spirit_id: spirit_id.to_owned(),
                version: version.to_owned(),
                admitted,
                effective_tier: effective_tier.to_owned(),
                journal_note: journal_note.to_owned(),
            },
        ),
    };
    let gov_bytes = match serde_json::to_vec(&gov_payload) {
        Ok(b) => b,
        // Near-infallible (serializing an internally-constructed governance
        // payload). On the impossible failure path, skip the TL write rather
        // than panic — this helper is best-effort governance logging.
        Err(_) => return,
    };
    let _token = tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::GovernanceEvent,
        0,
        None,
        if admitted {
            "governance:vetter-key-admission"
        } else {
            "governance:vetter-key-rejection"
        },
        &gov_bytes,
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
}

/// Story 9.4b AC-6/D7 — the deploy-time accountable operator identity stamped on
/// every model-provenance governance event. Resolved from
/// `MAOS_DEPLOYMENT_OPERATOR_ID`, defaulting to a stable single-operator
/// sentinel for v1.0 deployments (the ONLY persisted-record tenancy reservation).
fn deployment_operator_id() -> String {
    std::env::var("MAOS_DEPLOYMENT_OPERATOR_ID")
        .unwrap_or_else(|_| "maos.deployment.operator.default".to_string())
}

/// Story 9.4b AC-6 — resolve the model-provenance admission policy from the
/// operator environment. Defaults are AC-11 safe (provenance optional, no
/// staleness window) so pre-v3 / non-covered manifests stay admissible.
pub fn resolve_model_provenance_policy() -> maos_registry::admission::ModelProvenancePolicy {
    let require = std::env::var("MAOS_REQUIRE_MODEL_PROVENANCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let max_age_secs = std::env::var("MAOS_MODEL_PROVENANCE_MAX_AGE_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok());
    let now_unix_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    maos_registry::admission::ModelProvenancePolicy {
        require,
        max_age_secs,
        now_unix_secs,
    }
}

/// Story 9.4b AC-6 (D6/D7) — emit a `FrameKind::GovernanceEvent` carrying a
/// `ModelProvenancePayload` to the Transparency Log. Records the provenance
/// triple bound to schema-identity + content-hash with the constant
/// `deployment_operator_id` — SCHEMA identity only, zero claim-instance ids, so
/// it stays out of the GDPR forget cascade (D5). Queryable via
/// `maosctl audit query --kind governance`.
pub fn emit_model_provenance_event(
    tl: &maos_kernel_core::iac::TransparencyLogAdapter,
    rec: &maos_registry::admission::ModelProvenanceRecord,
) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(std::time::Duration::ZERO)
        .as_nanos() as u64;
    let gov_payload = maos_domain::governance::GovernanceEventPayload {
        recorded_at_ns: now,
        effective_at_ns: now,
        event: maos_domain::governance::GovernanceEventKind::ModelProvenance(
            maos_domain::governance::ModelProvenancePayload {
                schema_id: maos_domain::governance::MODEL_PROVENANCE_SCHEMA_ID.to_string(),
                schema_content_hash: rec.content_hash.clone(),
                deployment_operator_id: deployment_operator_id(),
                covered_model_id: rec.covered_model_id.clone(),
                training_data_lineage: rec.training_data_lineage.clone(),
                last_eval_timestamp: rec.last_eval_timestamp.clone(),
                version: 1,
            },
        ),
    };
    let gov_bytes = serde_json::to_vec(&gov_payload).map_err(|e| {
        format!("model-provenance governance serialization failed (fail-closed): {e}")
    })?;
    let _token = tl.insert_frame_event(
        maos_kernel_core::iac::transparency_log::FrameKind::GovernanceEvent,
        0,
        None,
        "governance:model-provenance-admission",
        &gov_bytes,
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
    Ok(())
}

/// Why a load was refused, as a typed outcome rather than a bare non-zero.
///
/// One variant per gate `maos run` applies (§2b), so AC1's "one refusal vector
/// per gate, each a distinct typed outcome" is discharged by exhaustiveness
/// rather than by a list someone maintains.
///
/// ⚠ There is deliberately NO `SandboxTierUnsupported` / `T3AdmissionFailed`
/// variant. The manifest's `[sandbox] tier` is parsed and then DISCARDED on
/// the admission path: `effective_sandbox_tier` seeds its manifest leg from
/// the policy table's per-pid declared tier, which is absent on a first
/// admission and therefore falls back to `SandboxTier::DEFAULT_FLOOR`, and
/// `admit_spirit` reads its `&SandboxConfig` only for `image_pin`. A
/// manifest cannot select its own tier, so it cannot be refused for one.
/// Those refusals arrive (if ever) inside `Admission`, from the kernel.
#[derive(Debug, Clone)]
pub enum AdmissionRefusal {
    /// Gate 1 — the manifest file could not be read.
    ManifestUnreadable { detail: String },
    /// Gate 2 — the manifest is not well-formed TOML.
    ManifestParse { detail: String },
    /// Gate 3 — `[cli_wrapper]` and `[class]` are mutually exclusive
    /// (architecture §6.7, `EManifestSchemaConflict`).
    CliWrapperClassConflict,
    /// Gate 3b — a `[cli_wrapper]` manifest has no class to load. Worker
    /// manifests are `maos run`'s business, not the door's.
    CliWrapperUnsupported,
    /// Gate 4 — `[class]` is absent or does not parse.
    ClassSection { detail: String },
    /// Gate 5 — the class name is not one of the eight compiled into this
    /// binary. `rust-inproc` means first-party compiled-in (ADR-060); the
    /// third-party form is Epic 17's `wasm-component`.
    UnknownClass { name: String },
    /// Gates 6 and 7 — a declared section did not parse.
    SectionParse {
        section: &'static str,
        detail: String,
    },
    /// Gate 8 — model-provenance admission (FR62).
    ModelProvenance { detail: String },
    /// The scheduler already holds this `spirit_id`; this is a state conflict,
    /// not a generic loader failure.
    AlreadyLoaded { spirit_id: String },
    /// The class-specific constructor could not build the requested Spirit.
    ClassBuild { detail: String },
    /// Mira and Researcher require their synchronous epistemic halt port.
    EpistemicPortRequired { detail: String },
    /// The scheduler refused the load for a reason other than duplicate id.
    Load { detail: String },
    /// Gate 11 — `SecurityManagerAdapter::admit_spirit` refused. Carries the
    /// ABI-window refusals (`EAbiTooOld` / `EAbiTooNew`, the NFR-Maint-9
    /// surface), `EClassRequired` and `ESubstrateTooOld`.
    Admission { detail: String },
    /// `on_start` failed, or the Spirit could not be started.
    Start { detail: String },
}

impl AdmissionRefusal {
    /// The stable machine code the door reports and the operator greps for.
    pub fn code(&self) -> &'static str {
        match self {
            Self::ManifestUnreadable { .. } => "manifest_unreadable",
            Self::ManifestParse { .. } => "manifest_parse_failed",
            Self::CliWrapperClassConflict => "manifest_schema_conflict",
            Self::CliWrapperUnsupported => "cli_wrapper_unsupported",
            Self::ClassSection { .. } => "class_section_invalid",
            Self::UnknownClass { .. } => "unknown_spirit_class",
            Self::SectionParse { .. } => "manifest_section_invalid",
            Self::ModelProvenance { .. } => "model_provenance_refused",
            Self::AlreadyLoaded { .. } => "already_loaded",
            Self::ClassBuild { .. } => "class_build_failed",
            Self::EpistemicPortRequired { .. } => "epistemic_port_required",
            Self::Load { .. } => "load_refused",
            Self::Admission { .. } => "admission_refused",
            Self::Start { .. } => "start_failed",
        }
    }

    /// True when the scheduler already holds this `spirit_id`. The door turns
    /// this into a `409`, distinct from every other refusal's `400`.
    pub fn is_already_loaded(&self) -> bool {
        matches!(self, Self::AlreadyLoaded { .. })
    }
}

impl std::fmt::Display for AdmissionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManifestUnreadable { detail } => write!(f, "manifest unreadable: {detail}"),
            Self::ManifestParse { detail } => write!(f, "manifest is not valid TOML: {detail}"),
            Self::CliWrapperClassConflict => f.write_str(
                "manifest declares both [cli_wrapper] and [class] — mutually exclusive \
                 (architecture §6.7, EManifestSchemaConflict)",
            ),
            Self::CliWrapperUnsupported => f.write_str(
                "[cli_wrapper] manifests declare no Spirit class and cannot be loaded over \
                 the operator door",
            ),
            Self::ClassSection { detail } => write!(f, "[class] section: {detail}"),
            Self::UnknownClass { name } => write!(
                f,
                "unknown Spirit class '{name}' (known: {}) — `rust-inproc` admits \
                 first-party classes compiled into this binary only (ADR-060)",
                KNOWN_CLASS_NAMES.join(", ")
            ),
            Self::SectionParse { section, detail } => {
                write!(f, "[{section}] section: {detail}")
            }
            Self::ModelProvenance { detail } => {
                write!(f, "model-provenance admission failed: {detail}")
            }
            Self::AlreadyLoaded { spirit_id } => write!(f, "spirit already loaded: {spirit_id}"),
            Self::ClassBuild { detail } => write!(f, "Spirit class construction failed: {detail}"),
            Self::EpistemicPortRequired { detail } => {
                write!(f, "epistemic halt port required: {detail}")
            }
            Self::Load { detail } => write!(f, "scheduler refused the load: {detail}"),
            Self::Admission { detail } => write!(f, "admission rejected: {detail}"),
            Self::Start { detail } => write!(f, "start failed: {detail}"),
        }
    }
}

/// The eight class names `maos-bin` can construct, in `classify_spirit` order.
///
/// Declared here rather than in `main.rs` so the library half of the load path
/// can name the bound in its refusal message; `main.rs` owns the mapping to
/// concrete Rust types and a parity test keeps the two from drifting.
pub const KNOWN_CLASS_NAMES: [&str; 8] = [
    "butler",
    "researcher",
    "orchestrator",
    "architect",
    "reviewer",
    "mira",
    "nash",
    "digest",
];

/// Every manifest section the admission triple needs, parsed and gate-checked.
#[derive(Debug, Clone)]
pub struct GatedManifest {
    pub spirit_id: String,
    pub class_section: ClassSection,
    pub sandbox_cfg: SandboxConfig,
    pub resource_caps: ResourceCaps,
    pub caps_required: CapabilitiesRequired,
    pub output_shape: OutputShape,
    pub posture_section: PostureSection,
    pub epistemic_policy: Option<EpistemicPolicySection>,
    pub scheduling: SchedulingSection,
    pub lifecycle: LifecycleSection,
    pub budget: Option<Budget>,
    /// The raw text, kept for the model-provenance gate, which parses its own
    /// section out of the whole document.
    pub manifest_toml: String,
}

impl GatedManifest {
    /// The bundle the scheduler stores in the SCB.
    pub fn bundle(&self) -> SpiritManifestBundle {
        SpiritManifestBundle {
            scheduling: self.scheduling.clone(),
            lifecycle: self.lifecycle.clone(),
            class: Some(self.class_section.clone()),
            budget: self.budget.clone(),
            ..Default::default()
        }
    }
}

/// The `spirit_id` a manifest will claim, without gating it.
///
/// The door reads the bounded, regular-file manifest once on a blocking worker,
/// then uses this total and silent parser only to choose its serialization key.
/// A malformed or invalid id deliberately returns `None`: the real gates own
/// the typed refusal.
pub fn peek_spirit_id_from_str(text: &str) -> Option<String> {
    let root: toml::Value = toml::from_str(text).ok()?;
    let spirit_id = root.get("class")?.get("name")?.as_str()?;
    (1..=128)
        .contains(&spirit_id.len())
        .then_some(spirit_id)
        .filter(|id| {
            id.bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
        .map(str::to_owned)
}

/// Gate 1 — read the manifest.
///
/// The path is already canonical: the CLI canonicalises before sending
/// (D-16-1-X — the daemon's working directory is not the operator's), and
/// `maos run` passes an operator-supplied path it resolves in its own CWD.
/// Neither caller canonicalises twice.
pub fn read_manifest(path: &Path) -> Result<String, AdmissionRefusal> {
    let meta = std::fs::metadata(path).map_err(|error| AdmissionRefusal::ManifestUnreadable {
        detail: format!("{}: {error}", path.display()),
    })?;
    if !meta.is_file() {
        return Err(AdmissionRefusal::ManifestUnreadable {
            detail: format!("{}: not a regular file", path.display()),
        });
    }
    if meta.len() > 1024 * 1024 {
        return Err(AdmissionRefusal::ManifestUnreadable {
            detail: "manifest exceeds 1 MiB limit".into(),
        });
    }
    std::fs::read_to_string(path).map_err(|error| AdmissionRefusal::ManifestUnreadable {
        detail: format!("{}: {error}", path.display()),
    })
}

/// Gates 2–7 — parse the document and every section the triple needs.
///
/// `known_class` is the caller's class predicate (`classify_spirit` in
/// `main.rs`, which owns the mapping to concrete Rust types). Passing it keeps
/// gate 5 in this one ordering without duplicating the class table.
pub fn gate_manifest(
    manifest_toml: &str,
    known_class: &dyn Fn(&str) -> bool,
) -> Result<GatedManifest, AdmissionRefusal> {
    let root: toml::Value =
        toml::from_str(manifest_toml).map_err(|error| AdmissionRefusal::ManifestParse {
            detail: error.to_string(),
        })?;

    // Gate 3 — the `[cli_wrapper]` fork, BEFORE `[class]` is extracted.
    if root.get("cli_wrapper").is_some() {
        if root.get("class").is_some() {
            return Err(AdmissionRefusal::CliWrapperClassConflict);
        }
        return Err(AdmissionRefusal::CliWrapperUnsupported);
    }

    // Same shape `maos run`'s `extract`/`opt_section` closures use: the
    // section VALUE is serialized, not a synthetic one-key document.
    let section = |name: &str| -> Option<String> {
        root.get(name).and_then(|value| toml::to_string(value).ok())
    };
    let required = |name: &'static str| -> Result<String, AdmissionRefusal> {
        section(name).ok_or(AdmissionRefusal::SectionParse {
            section: name,
            detail: "section is required and absent".into(),
        })
    };
    let parse_err = |name: &'static str| {
        move |error: String| AdmissionRefusal::SectionParse {
            section: name,
            detail: error,
        }
    };

    // Gate 4 — `[class]` present and parsing.
    let class_toml = section("class").ok_or(AdmissionRefusal::ClassSection {
        detail: "section is required and absent".into(),
    })?;
    let class_section = ClassSection::from_toml_str(&class_toml).map_err(|error| {
        AdmissionRefusal::ClassSection {
            detail: error.to_string(),
        }
    })?;

    // Gate 5 — a class this binary can actually construct.
    if !known_class(&class_section.name) {
        return Err(AdmissionRefusal::UnknownClass {
            name: class_section.name.clone(),
        });
    }

    // Gate 6 — the mediation sections.
    let sandbox_cfg = SandboxConfig::from_toml_str(&required("sandbox")?)
        .map_err(|e| parse_err("sandbox")(e.to_string()))?;
    let resource_caps = ResourceCaps::from_toml_str(&required("resources")?)
        .map_err(|e| parse_err("resources")(e.to_string()))?;
    let caps_required = caps_required_or_empty(&root)?
        .degrade_for_schema_version(class_section.manifest_schema_version);
    let output_shape = OutputShape::from_toml_str(&required("output_shape")?)
        .map_err(|e| parse_err("output_shape")(e.to_string()))?;
    let posture_section = PostureSection::from_toml_str(&required("posture")?)
        .map_err(|e| parse_err("posture")(e.to_string()))?;

    // Gate 7 — the optional behavioural sections.
    let epistemic_policy = section("epistemic_policy")
        .map(|s| {
            EpistemicPolicySection::from_toml_str(&s)
                .map_err(|e| parse_err("epistemic_policy")(e.to_string()))
        })
        .transpose()?;
    // Optional for the reference cognitive Spirits: they fire `on_idle`, not
    // scheduled hooks, and an empty `enabled_hooks` allows every hook.
    let scheduling = match section("scheduling") {
        Some(s) => SchedulingSection::from_toml_str(&s)
            .map_err(|e| parse_err("scheduling")(e.to_string()))?,
        None => SchedulingSection::default(),
    };
    let lifecycle = match section("lifecycle") {
        Some(s) => LifecycleSection::from_toml_str(&s)
            .map_err(|e| parse_err("lifecycle")(e.to_string()))?,
        None => LifecycleSection::default(),
    };
    let budget = section("budget")
        .map(|s| Budget::from_toml_str(&s).map_err(|e| parse_err("budget")(e.to_string())))
        .transpose()?;

    Ok(GatedManifest {
        spirit_id: class_section.name.clone(),
        class_section,
        sandbox_cfg,
        resource_caps,
        caps_required,
        output_shape,
        posture_section,
        epistemic_policy,
        scheduling,
        lifecycle,
        budget,
        manifest_toml: manifest_toml.to_owned(),
    })
}

/// `[capabilities.required]`, or the empty set. Absent capabilities are a
/// legitimate manifest shape (a Spirit that needs nothing), not a refusal —
/// this mirrors `maos run`'s own `caps_required_or_empty`.
fn caps_required_or_empty(root: &toml::Value) -> Result<CapabilitiesRequired, AdmissionRefusal> {
    let fail = |error: String| AdmissionRefusal::SectionParse {
        section: "capabilities.required",
        detail: error,
    };
    match root.get("capabilities").and_then(|c| c.get("required")) {
        Some(value) => {
            let text = toml::to_string(value).map_err(|e| fail(e.to_string()))?;
            CapabilitiesRequired::from_toml_str(&text).map_err(|e| fail(e.to_string()))
        }
        None => Ok(CapabilitiesRequired {
            provider: maos_kernel_core::security::ProviderCapabilities {
                complete: Vec::new(),
            },
            mcp: maos_kernel_core::security::McpCapabilities {
                servers: Vec::new(),
            },
            loom: maos_kernel_core::security::manifest::LoomCapabilities::default(),
        }),
    }
}

/// The daemon-side composites the triple needs. Deliberately a borrow bundle:
/// the door's `BinPrivateOps` impl and `maos run`'s composition root both
/// already own these, and neither should clone the world to call one function.
pub struct AdmissionDeps<'a> {
    pub scheduler: &'a Arc<SpiritSchedulerAdapter>,
    pub security: &'a Arc<SecurityManagerAdapter>,
    pub transparency_log: &'a Arc<maos_kernel_core::iac::TransparencyLogAdapter>,
    pub journal: &'a Arc<maos_kernel_core::journal::JournalAdapter>,
    pub memory: &'a Arc<maos_kernel_core::memory::MemoryManagerAdapter>,
    pub pid_by_spirit_id: &'a Arc<RwLock<BTreeMap<String, u32>>>,
}

/// Whether the triple runs its third leg.
///
/// `maos run` boots a Spirit: load, admit, start, in one breath. `maosctl
/// load` deliberately stops at `Loaded`, because FR9 names FIVE verbs and
/// collapsing `load` and `start` into one would leave `maosctl start` with no
/// operator-creatable subject — the state it has been in since 16-1 shipped
/// it. The two callers differ in exactly this one bit; everything else about
/// the path, including the rollback, is shared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPolicy {
    /// `maos run` — the Spirit is running when the call returns.
    StartNow,
    /// `maosctl load` — the Spirit is `Loaded` and the operator owns the
    /// decision to `start` it or `unload` it.
    LeaveLoaded,
}

/// What the operator learns about a Spirit this call admitted.
#[derive(Debug, Clone)]
pub struct Admitted {
    pub pid: u32,
    pub spirit_id: String,
    pub effective_sandbox_tier: maos_domain::invariants::i9::SandboxTier,
    /// The `[posture] allowed_max` the manifest asked for.
    pub requested_posture_ceiling: Posture,
    /// The ceiling actually in force after the operator clamp. When the two
    /// differ, the operator's policy lowered it — and AC1 requires that to be
    /// readable, not inferable.
    pub effective_posture_ceiling: Posture,
}

/// The whole `provenance → load → admit → start` triple, with rollback.
///
/// `spirit_obj` is already class-dispatched by the caller, which is why the
/// scheduler's type-erased `load_obj` exists: the standalone arm builds
/// Spirits with run-specific wiring (butler's output channel and boot-loud
/// scalar port, researcher's collective and inference bindings, mira's scalar
/// port) that no shared function could reconstruct, while the door and the
/// topology loop build plain ones. Sharing the *triple* is the point; sharing
/// the *constructors* is neither possible nor desirable.
///
/// `on_loaded` runs once the scheduler has assigned the pid and before
/// admission — it is where callers re-point the pid-keyed handles they built
/// at construction time with a placeholder (`LiveButlerMcpPort::spirit_pid`,
/// the scalar-port pid bindings). Safe there: those ports are only reached
/// from `on_idle`, which cannot fire before `start`.
pub async fn load_admit_start(
    deps: &AdmissionDeps<'_>,
    gated: &GatedManifest,
    spirit_obj: Arc<dyn AnySpiritObj>,
    boot_nonce: u64,
    start: StartPolicy,
    on_loaded: &mut (dyn FnMut(u32) + Send),
) -> Result<Admitted, AdmissionRefusal> {
    let spirit_id = gated.spirit_id.clone();

    // Gate 8 — model provenance, BEFORE a pid is allocated. Refusing after
    // the load is what the topology arm did, and it meant a provenance-refused
    // Spirit had already been admitted at a real pid.
    if let Some(record) = maos_registry::admission::validate_model_provenance(
        gated.manifest_toml.as_bytes(),
        &resolve_model_provenance_policy(),
    )
    .map_err(|error| AdmissionRefusal::ModelProvenance {
        detail: error.to_string(),
    })? {
        emit_model_provenance_event(deps.transparency_log, &record)
            .map_err(|detail| AdmissionRefusal::ModelProvenance { detail })?;
    }

    let pid = match deps
        .scheduler
        .load_obj(&spirit_id, gated.bundle(), spirit_obj, boot_nonce)
        .await
    {
        Ok(pid) => pid,
        Err(maos_domain::lifecycle::LifecycleError::AlreadyLoaded { spirit_id }) => {
            return Err(AdmissionRefusal::AlreadyLoaded { spirit_id });
        }
        Err(error) => {
            if let Some(pid) = deps.scheduler.resolve_pid(&spirit_id) {
                rollback(deps, pid, &spirit_id).await;
            }
            return Err(AdmissionRefusal::Load {
                detail: error.to_string(),
            });
        }
    };

    on_loaded(pid);

    // Admission runs at the SCHEDULER-ASSIGNED pid: capability mediation is
    // keyed by pid, so admitting a placeholder would fail every live port
    // closed.
    let spec = match deps.security.admit_spirit(
        pid,
        &spirit_id,
        &gated.sandbox_cfg,
        &gated.resource_caps,
        &gated.caps_required,
        Some(&gated.output_shape),
        deps.journal.as_ref(),
        &gated.posture_section,
        gated.epistemic_policy.as_ref(),
        None,
        None,
        None,
        None,
        None,
        Some(&gated.class_section),
    ) {
        Ok(spec) => spec,
        Err(error) => {
            // The vetter event fires on REJECTION as well as grant. The
            // topology arm emitted on grant only, so a refused admission left
            // no audit row at all — unauditable, and the reason this arm is
            // shared rather than duplicated.
            emit_vetter_key_event(
                deps.transparency_log,
                &spirit_id,
                &gated.class_section.version,
                false,
                "N/A",
                &format!("admission rejected: {error}"),
            );
            rollback(deps, pid, &spirit_id).await;
            return Err(AdmissionRefusal::Admission {
                detail: error.to_string(),
            });
        }
    };
    emit_vetter_key_event(
        deps.transparency_log,
        &spirit_id,
        &gated.class_section.version,
        true,
        &format!("{:?}", spec.tier),
        "admission granted",
    );

    deps.pid_by_spirit_id
        .write()
        .unwrap_or_else(|error| {
            eprintln!("CRITICAL: pid_by_spirit_id RwLock poisoned (admission insert)");
            error.into_inner()
        })
        .insert(spirit_id.clone(), pid);

    if start == StartPolicy::StartNow {
        if let Err(error) = deps.scheduler.start(pid).await {
            if matches!(
                &error,
                maos_domain::lifecycle::LifecycleError::HookPanicked { .. }
            ) {
                // Story 16-6 review 2026-09-20: `handle_crash` already owns
                // teardown after a panicked hook. A lost race can only lose
                // crash classification, never corrupt scheduler state.
                remove_pid_by_spirit_id(deps, &spirit_id);
            } else {
                rollback(deps, pid, &spirit_id).await;
            }
            return Err(AdmissionRefusal::Start {
                detail: error.to_string(),
            });
        }
    }

    // SR-1: authorize this admitted Spirit for principal namespace writes.
    // Unconditional: the Spirit is admitted either way, and a later `start`
    // over the door does not revisit admission.
    deps.memory.authorize_principal_writes(pid);

    let effective_posture_ceiling = deps
        .security
        .policy()
        .inner()
        .load_full()
        .spirit_postures
        .get(&pid)
        .map(|state| state.allowed_max)
        .unwrap_or(gated.posture_section.allowed_max);

    Ok(Admitted {
        pid,
        spirit_id,
        effective_sandbox_tier: spec.tier,
        requested_posture_ceiling: gated.posture_section.allowed_max,
        effective_posture_ceiling,
    })
}

/// Unwind a load whose admission or start failed.
///
/// A refusal must leave NOTHING behind: no SCB, no pid-by-id entry, no live
/// tokens. This is only expressible because the kernel gained
/// `(Loaded, Unloaded)` — before it, `scheduler.unload` on a never-started
/// Spirit returned `InvalidStateTransition` and the SCB leaked permanently,
/// burning the `spirit_id` for the lifetime of the daemon.
async fn rollback(deps: &AdmissionDeps<'_>, pid: u32, spirit_id: &str) {
    if let Err(error) = deps.scheduler.unload(pid).await {
        eprintln!(
            "maos: CRITICAL — rollback of partially admitted Spirit '{spirit_id}' (pid {pid}) \
             failed: {error}"
        );
    }
    remove_pid_by_spirit_id(deps, spirit_id);
}

/// Remove the door's lookup only; panic teardown retains scheduler ownership.
fn remove_pid_by_spirit_id(deps: &AdmissionDeps<'_>, spirit_id: &str) {
    deps.pid_by_spirit_id
        .write()
        .unwrap_or_else(|error| {
            eprintln!("maos: CRITICAL — pid_by_spirit_id RwLock poisoned (admission removal)");
            error.into_inner()
        })
        .remove(spirit_id);
}
