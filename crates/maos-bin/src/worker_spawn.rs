/// The `maos run` **worker-spawn surface** — `[cli_wrapper]` admission, the
/// host-grant allowlist, the enterprise-governed capability mint, and the real
/// subprocess bridge.
///
/// In the library, not `main.rs`, for the reason `worker_cli` moved in
/// j1-crosshost-2a (`lib.rs:18-25`) and `topology` moved in 1a (`lib.rs:27-30`):
/// **in-process item visibility**. `crates/maos-bin/tests/` cannot NAME a private
/// item of the binary crate, so a host-B proof could not make a typed
/// `WorkerCompletion` assertion or inject a port — it could only drive the whole
/// binary by subprocess (which `worker_completion_2a.rs:871` already does).
/// Subprocess coverage is not the gap; typed in-process coverage is.
///
/// This module is a RELOCATION, not a rewrite (j1-crosshost-2b AC1.1): every item
/// below is byte-identical to its `main.rs` original apart from visibility
/// (`fn` → `pub fn`) and crate-internal path rewrites (`maos_bin::` → `crate::`).
/// The `cohort-a2a-daemon` region deliberately did NOT move — two suites assert
/// its literal text inside `include_str!("../src/main.rs")` (Trap 9).
use std::sync::Arc;

use crate::{enterprise_identity, enterprise_pdp_runtime, worker_cli};
use maos_domain::invariants::i1::{CapabilityToken, IntentClass, Scope};
use maos_domain::ports::{scope_action_key, PolicyVerdict};
use maos_kernel_core::api::CapabilityRegistryAdapter;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind};

/// Map a finalized bridge result to the worker CLI's completion input.
///
/// The binding's lifecycle decision takes precedence over retained exit facts;
/// unavailable T3 facts never become a successful completion marker.
pub fn worker_exit_from_finalized_bridge(
    may_complete: bool,
    exit: &maos_kernel_core::lifecycle::cli_wrapper::runtime::BridgeExit,
) -> worker_cli::WorkerExit {
    if !may_complete {
        worker_cli::WorkerExit::Crashed
    } else if let Some((facts, _)) = exit.container.as_ref() {
        facts
            .completion_exit_code()
            .map(worker_cli::WorkerExit::Exited)
            .unwrap_or(worker_cli::WorkerExit::Crashed)
    } else {
        match exit.cause.exit_code() {
            Some(code) => worker_cli::WorkerExit::Exited(code),
            None => worker_cli::WorkerExit::Crashed,
        }
    }
}

#[cfg(feature = "network")]
/// Parsed `maos run` invocation. `None` (from [`parse_run_args`]) means no `run`
/// subcommand was given → preserve the existing `MAOS_ONE_SHOT` / Spirit-less
/// serving behavior.
#[cfg(feature = "network")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunArgs {
    pub manifest_path: String,
    /// `--live` → real Inference provider; absent → deterministic replay/stub.
    pub live: bool,
    /// `--once` → single `on_idle` pass + graceful drain (headless tests).
    pub once: bool,
    /// Story 18-4 (CR-2) — `--clock compressed:<n>d --acceptance <path>`: a
    /// compressed Butler scene, one `on_idle` per simulated day.
    pub scene: Option<SceneArgs>,
}

/// Story 18-4 (CR-2) — the compressed scene's clock and acceptance fixture.
#[cfg(feature = "network")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneArgs {
    pub clock: maos_spirit_sdk::clock::ClockSpec,
    pub acceptance: String,
}

#[cfg(feature = "network")]
const RUN_USAGE: &str =
    "<manifest> [--live] [--once] [--clock compressed:<n>d --acceptance <path>]";

#[cfg(feature = "network")]
/// Parse `run <manifest-path> [--live] [--once] [--clock <spec> --acceptance
/// <path>]` from the process args (the args AFTER the binary name). Manual
/// parsing — the binary has no clap dependency. Returns `None` when the
/// first arg is not `run` (the env-gated paths win). Story 18-4 (CR-2): the
/// scene flags come as a pair, never with `--once` or `--live`.
pub fn parse_run_args<I: IntoIterator<Item = String>>(args: I) -> Result<Option<RunArgs>, String> {
    let mut it = args.into_iter();
    if it.next().as_deref() != Some("run") {
        return Ok(None);
    }
    let mut manifest_path: Option<String> = None;
    let (mut live, mut once) = (false, false);
    let (mut clock, mut acceptance): (Option<String>, Option<String>) = (None, None);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--live" => live = true,
            "--once" => once = true,
            "--clock" | "--acceptance" => {
                let value = it.next().ok_or_else(|| {
                    format!("maos run: {a} needs a value — expected: {RUN_USAGE}")
                })?;
                *(if a == "--clock" {
                    &mut clock
                } else {
                    &mut acceptance
                }) = Some(value);
            }
            other if !other.starts_with("--") && manifest_path.is_none() => {
                manifest_path = Some(other.to_string());
            }
            other => {
                return Err(format!(
                    "maos run: unknown argument '{other}' — expected: {RUN_USAGE}"
                ));
            }
        }
    }
    let scene = match (clock, acceptance) {
        (None, None) => None,
        (Some(_), None) => return Err("maos run: --clock requires --acceptance <path>".into()),
        (None, Some(_)) => return Err("maos run: --acceptance requires --clock <spec>".into()),
        (Some(_), Some(_)) if once => {
            return Err(
                "maos run: --clock/--acceptance run a scene; they cannot be combined with --once"
                    .into(),
            )
        }
        (Some(_), Some(_)) if live => return Err(
            "maos run: --clock/--acceptance replay a scene; they cannot be combined with --live"
                .into(),
        ),
        (Some(clock), Some(acceptance)) => Some(SceneArgs {
            clock: maos_spirit_sdk::clock::ClockSpec::parse(&clock)
                .map_err(|e| format!("maos run: --clock: {e}"))?,
            acceptance,
        }),
    };
    match manifest_path {
        Some(manifest_path) => Ok(Some(RunArgs {
            manifest_path,
            live,
            once,
            scene,
        })),
        None => Err(format!(
            "maos run: missing manifest path — expected: maos run {RUN_USAGE}"
        )),
    }
}

/// Story 8.12 — map a `[sandbox] tier = "T3"` string to the operational tier.
#[cfg(feature = "network")]
pub fn parse_sandbox_tier(s: &str) -> Result<maos_domain::invariants::i9::SandboxTier, String> {
    use maos_domain::invariants::i9::SandboxTier;
    match s.trim().to_ascii_uppercase().as_str() {
        "T0" => Ok(SandboxTier::T0),
        "T1" => Ok(SandboxTier::T1),
        "T2" => Ok(SandboxTier::T2),
        "T3" => Ok(SandboxTier::T3),
        "T4" => Ok(SandboxTier::T4),
        other => Err(format!("maos run: unknown sandbox tier '{other}'")),
    }
}

/// Story 8.12 — resolve a CliWrapper `command` to a runnable path. The fixture
/// binaries are built in the daemon's Cargo target directory; integration tests
/// run from `target/debug/deps/`, so the target directory parent is checked too.
#[cfg(feature = "network")]
pub fn resolve_cli_binary(command: &str) -> Result<String, String> {
    let p = std::path::Path::new(command);
    if p.is_absolute() {
        return if p.exists() {
            Ok(command.to_string())
        } else {
            Err(format!(
                "maos run: cli_wrapper command not found at absolute path '{command}'"
            ))
        };
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let cand = dir.join(command);
            if cand.is_file() {
                return Ok(cand.to_string_lossy().into_owned());
            }
            if let Some(up) = dir.parent() {
                let cand2 = up.join(command);
                if cand2.is_file() {
                    return Ok(cand2.to_string_lossy().into_owned());
                }
            }
        }
    }
    if let Some(pathv) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&pathv) {
            let c = d.join(command);
            if c.is_file() {
                return Ok(c.to_string_lossy().into_owned());
            }
        }
    }
    Err(format!(
        "maos run: cli_wrapper command '{command}' not found (checked daemon-sibling, deps/ parent, and $PATH)"
    ))
}

/// The built-in HOST grant for the hermetic fixture Worker.
#[cfg(feature = "network")]
pub fn builtin_fixture_grant() -> maos_domain::host_grant::HostGrant {
    maos_domain::host_grant::HostGrant {
        attested_image: "worker-cli-fixture".to_string(),
        signing_key_id: "MAOS Project".to_string(),
        permitted_tier: maos_domain::invariants::i9::SandboxTier::T3,
        permitted_egress_destinations: vec![],
    }
}

/// The probe receives the same T3 admission authority with no host-granted
/// egress permission.
#[cfg(feature = "network")]
pub fn builtin_probe_grant() -> maos_domain::host_grant::HostGrant {
    maos_domain::host_grant::HostGrant {
        attested_image: worker_cli::PROBE_CLI_NAME.to_string(),
        signing_key_id: "MAOS Project".to_string(),
        permitted_tier: maos_domain::invariants::i9::SandboxTier::T3,
        permitted_egress_destinations: vec![],
    }
}

/// Parse an operator `MAOS_HOST_GRANTS` TOML file into host grants. Schema:
/// ```toml
/// [[grant]]
/// attested_image = "codex"
/// signing_key_id = "OpenAI"
/// permitted_tier = "T3"
/// permitted_egress_destinations = ["api.openai.com"]
/// ```
/// A grant missing a required field is an ERROR — never a silent admit.
#[cfg(feature = "network")]
pub fn parse_host_grants_toml(
    text: &str,
) -> Result<Vec<maos_domain::host_grant::HostGrant>, String> {
    let root: toml::Value = toml::from_str(text).map_err(|e| format!("toml parse: {e}"))?;
    let Some(arr) = root.get("grant").and_then(|g| g.as_array()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(arr.len());
    for (i, g) in arr.iter().enumerate() {
        let attested_image = g
            .get("attested_image")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("grant[{i}]: missing attested_image"))?
            .to_string();
        let signing_key_id = g
            .get("signing_key_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("grant[{i}]: missing signing_key_id"))?
            .to_string();
        let permitted_tier = match g.get("permitted_tier").and_then(|v| v.as_str()) {
            Some(s) => parse_sandbox_tier(s)?,
            None => maos_domain::invariants::i9::SandboxTier::T3,
        };
        let mut permitted_egress_destinations = Vec::new();
        if let Some(value) = g.get("permitted_egress_destinations") {
            let entries = value.as_array().ok_or_else(|| {
                format!("grant[{i}].permitted_egress_destinations: expected an array")
            })?;
            for (j, entry) in entries.iter().enumerate() {
                let field = format!("grant[{i}].permitted_egress_destinations[{j}]");
                let value = entry
                    .as_str()
                    .ok_or_else(|| format!("{field}: expected a string"))?;
                permitted_egress_destinations.push(
                    maos_domain::host_grant::EgressDestination::parse_granted(value)
                        .map_err(|error| format!("{field}: {error}"))?,
                );
            }
        }
        out.push(maos_domain::host_grant::HostGrant {
            attested_image,
            signing_key_id,
            permitted_tier,
            permitted_egress_destinations,
        });
    }
    Ok(out)
}

/// Load the HOST-MANAGED grant allowlist (replaces the v0.9 self-grant). Built-in
/// grants cover only the hermetic fixture and its security probe; the operator
/// adds real agent-CLI grants (codex/claude + egress) via `MAOS_HOST_GRANTS`.
/// A manifest whose (image, author) matches no grant fails CLOSED at
/// `resolve_cli_wrapper_tier` — never self-granted.
///
/// **Disposition of the unreadable/unparseable branch, stated rather than
/// inherited (j1-crosshost-2a AC2.4).** A `MAOS_HOST_GRANTS` file that cannot be
/// read or parsed does NOT abort the run: it warns and continues with built-in
/// grants only. The result is safe: real agent CLIs then fail closed at
/// `resolve_cli_wrapper_tier`. It remains a warning because tightening it to a
/// hard refusal changes the failure mode of every hermetic run that happens to
/// have a stale variable exported.
#[cfg(feature = "network")]
pub fn load_host_grant_allowlist() -> maos_domain::host_grant::StaticHostGrantAllowlist {
    let mut grants = vec![builtin_fixture_grant(), builtin_probe_grant()];
    if let Some(path) = std::env::var_os("MAOS_HOST_GRANTS") {
        let display = std::path::Path::new(&path).display().to_string();
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse_host_grants_toml(&text) {
                Ok(mut extra) => grants.append(&mut extra),
                Err(e) => eprintln!(
                    "maos run: MAOS_HOST_GRANTS parse error ({display}): {e}; \
                     using built-in grants only (real agent CLIs will fail closed)"
                ),
            },
            Err(e) => eprintln!(
                "maos run: MAOS_HOST_GRANTS unreadable ({display}): {e}; \
                 using built-in grants only (real agent CLIs will fail closed)"
            ),
        }
    }
    maos_domain::host_grant::StaticHostGrantAllowlist::new(grants)
}

#[cfg(feature = "network")]
fn principal_attributes_for_pdp(
    principal: &maos_domain::ports::AuthenticatedPrincipal,
) -> std::collections::HashMap<String, String> {
    let mut attrs = principal.attributes.clone();
    attrs
        .entry("sub".to_string())
        .or_insert_with(|| principal.subject.clone());
    attrs
        .entry("iss".to_string())
        .or_insert_with(|| principal.issuer.clone());
    attrs
        .entry("aud".to_string())
        .or_insert_with(|| principal.audience.clone());
    attrs
}

/// §A6 review P16 (decision D1, ratified 2026-08-17) — WHY the governed mint
/// failed. The remote-requested (host B) spawn path must distinguish a
/// GOVERNANCE DENIAL (PDP refused, or SSO configured-but-absent) from the
/// kernel-policy grant simply not existing yet (`proc.exec` is Epic 9 surface;
/// the AC5 host grant is the ratified stronger authority there). A denial is a
/// REFUSAL on the receiving path; a missing kernel policy grant is 2a's FORK B
/// loud-proceed on both paths.
#[cfg(feature = "network")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernedMintError {
    /// `MAOS_SSO_ASSERTION` absent while SSO is configured.
    SsoAssertionMissing(String),
    /// The enterprise PDP returned `PolicyVerdict::Deny`.
    PdpDenied(String),
    /// The kernel policy evaluated the requested scope and denied issuance.
    KernelPolicyDenied,
    /// Story 17-9 — the Spirit's capability issuance quota for the current
    /// window is exhausted (`CapError::IssuanceQuotaExhausted`).
    QuotaExhausted,
    /// Story 18-5 — the Spirit's context is exhausted (`CapError::ContextExhausted`).
    ContextExhausted,
    /// Other mediation or identity-persistence failure.
    Mediation(String),
}

#[cfg(feature = "network")]
impl std::fmt::Display for GovernedMintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KernelPolicyDenied => write!(f, "kernel policy denied capability issuance"),
            Self::QuotaExhausted => write!(f, "kernel capability issuance quota exhausted"),
            Self::ContextExhausted => write!(f, "kernel refused issuance: context exhausted"),
            Self::SsoAssertionMissing(m) | Self::PdpDenied(m) | Self::Mediation(m) => {
                write!(f, "{m}")
            }
        }
    }
}

#[cfg(feature = "network")]
impl From<GovernedMintError> for String {
    fn from(error: GovernedMintError) -> Self {
        error.to_string()
    }
}

#[cfg(feature = "network")]
pub fn issue_enterprise_governed_capability(
    capability: &CapabilityRegistryAdapter,
    enterprise_runtime: Option<&enterprise_identity::EnterpriseRuntime>,
    enterprise_pdp_runtime: Option<&enterprise_pdp_runtime::EnterprisePdpRuntime>,
    spirit_pid: u32,
    scope: Scope,
    ttl_secs: u32,
    posture_hash: [u8; 32],
    intent_class: IntentClass,
) -> Result<CapabilityToken, GovernedMintError> {
    let capability_key = scope_action_key(&scope).to_string();
    let principal = match enterprise_runtime {
        Some(runtime) if runtime.sso_configured() => {
            let assertion = std::env::var("MAOS_SSO_ASSERTION").map_err(|_| {
                GovernedMintError::SsoAssertionMissing(format!(
                    "enterprise SSO is configured but MAOS_SSO_ASSERTION is absent for {capability_key}"
                ))
            })?;
            runtime
                .verify_principal_for_issuance(spirit_pid, &assertion)
                .map_err(|e| GovernedMintError::SsoAssertionMissing(e.to_string()))?
        }
        _ => None,
    };

    if let Some(pdp) = enterprise_pdp_runtime {
        let principal_attributes = principal.as_ref().map(principal_attributes_for_pdp);
        match pdp
            .evaluate_issuance(spirit_pid, &capability_key, principal_attributes)
            .map_err(|e| {
                GovernedMintError::PdpDenied(format!(
                    "enterprise PDP issuance evaluation failed: {e}"
                ))
            })? {
            PolicyVerdict::Allow => {}
            PolicyVerdict::Deny => {
                return Err(GovernedMintError::PdpDenied(format!(
                    "enterprise PDP denied capability issuance for {capability_key}"
                )));
            }
        }
    }

    let token = capability
        .issue_with_mediation(spirit_pid, scope, ttl_secs, posture_hash, intent_class)
        .map_err(|e| match e {
            maos_domain::ports::capability::CapError::PolicyDenied => {
                GovernedMintError::KernelPolicyDenied
            }
            maos_domain::ports::capability::CapError::IssuanceQuotaExhausted { .. } => {
                GovernedMintError::QuotaExhausted
            }
            maos_domain::ports::capability::CapError::ContextExhausted { .. } => {
                GovernedMintError::ContextExhausted
            }
            _ => GovernedMintError::Mediation(format!("kernel capability mediation failed: {e}")),
        })?;

    if let (Some(runtime), Some(principal)) = (enterprise_runtime, principal.as_ref()) {
        if let Err(error) =
            runtime.persist_identity_asserted(spirit_pid, principal, &capability_key)
        {
            maos_domain::ports::capability::CapabilityRegistryPort::revoke(
                capability,
                token.token_id,
            )
            .map_err(|revoke_error| {
                GovernedMintError::Mediation(format!(
                    "identity assertion persistence failed: {error}; token rollback failed: {revoke_error}"
                ))
            })?;
            return Err(GovernedMintError::Mediation(error.to_string()));
        }
    }

    Ok(token)
}

/// Story 17-9 CR-5 — the governed mint of one `McpCall` token for a Spirit's
/// MCP call. On ANY governed refusal (SSO, PDP, kernel policy, issuance/context
/// exhaustion, identity-persistence rollback) it writes exactly one kind-28
/// `mcp.call.refused:<code>` receipt at the Spirit's pid and
/// trusted id before returning the typed error; a success writes no receipt.
/// Callers return before `McpClientPort::call` on `Err`. Receipts are kind 28,
/// outside the Spirit-disclosable raw kinds, so a Spirit never recalls them.
#[cfg(feature = "network")]
#[allow(clippy::too_many_arguments)]
pub fn issue_governed_mcp_capability(
    capability: &CapabilityRegistryAdapter,
    enterprise_runtime: Option<&enterprise_identity::EnterpriseRuntime>,
    enterprise_pdp_runtime: Option<&enterprise_pdp_runtime::EnterprisePdpRuntime>,
    journal: &maos_kernel_core::iac::transparency_log::TransparencyLogAdapter,
    spirit_pid: u32,
    spirit_id: &str,
    server: &str,
    tool: &str,
    posture_hash: [u8; 32],
) -> Result<CapabilityToken, GovernedMintError> {
    let result = issue_enterprise_governed_capability(
        capability,
        enterprise_runtime,
        enterprise_pdp_runtime,
        spirit_pid,
        Scope::McpCall {
            server: server.into(),
            tool: tool.into(),
        },
        60,
        posture_hash,
        IntentClass::Standard,
    );
    if let Err(error) = &result {
        let code = match error {
            GovernedMintError::ContextExhausted => "context-exhausted",
            _ => "capability-denied",
        };
        journal.record_mcp_refusal(spirit_pid, Some(spirit_id), server, tool, code);
    }
    result
}

#[cfg(feature = "network")]
fn resolve_worker_caps(
    manifest_root: &toml::Value,
    policy: &maos_kernel_core::capability::cap_policy::PolicyTable,
) -> Result<maos_manifest::ResolvedCaps, String> {
    let manifest_caps = match manifest_root.get("resources") {
        Some(resources) => {
            let text = toml::to_string(resources)
                .map_err(|error| format!("maos run: serialize [resources]: {error}"))?;
            maos_manifest::ResourceCaps::from_toml_str(&text)
                .map_err(|error| format!("maos run: [resources] parse: {error}"))?
        }
        None => maos_manifest::ResourceCaps::default(),
    };

    // Read the live copy-on-write policy for every Worker launch. Passing the
    // existing authority here keeps admission resource-only; the existing full
    // capability admission remains the sole admission system.
    let snapshot = policy.inner().load_full();
    let empty_operator_caps = maos_manifest::ResourceCaps::default();
    let operator_caps = snapshot
        .operator_policy
        .resource_cap_floor
        .as_ref()
        .unwrap_or(&empty_operator_caps);
    Ok(maos_manifest::resolve_caps(&manifest_caps, operator_caps))
}

/// j1-crosshost-1a AC1.6 — `DEFAULT_WORKER_TASK` and the `MAOS_WORKER_TASK` read
/// are DELETED. The Worker's task is frame-borne: `delegated_task` carries the
/// `goal` drained from the delegation `task.assign` payload, and `None` means
/// there was no delegation (the standalone `[cli_wrapper]` path), not a default.
///
/// Returns the adapter-parsed completion label so the caller can journal it as a
/// real `FrameKind::TaskComplete` frame (AC3.10).
///
/// # Story 16-3 — the Worker is SUPERVISED (D-16-3-C, D-16-3-D)
///
/// `supervision` is the port ([`crate::supervision::WorkerSupervision`]); `task`
/// says WHERE this run came from so `bind` can derive the in-flight task
/// record without inventing a task id. Between them the Worker gets a real SCB
/// at a real pid: every row this function writes moves off pid 0, a SIGKILL
/// reaches `handle_crash` through the exit observer instead of waiting for the
/// last pipe holder, and a stopped or crashed Worker's task is dispositioned.
///
/// This function STAYS SYNC (19-3 AC3 relies on it): the port does the
/// `block_on`ing, which is why paths (a)/(b) call it inside
/// `tokio::task::block_in_place` and path (c) already runs inside
/// `spawn_blocking`.
///
/// **Every return between `bind` and `finish` is covered by the guard's
/// `Drop`** — it signals the child, dispositions the record and unloads the
/// SCB. That is structural, not a convention: a `?` added here later cannot
/// leak a bound Worker.
#[cfg(feature = "network")]
#[allow(clippy::too_many_arguments)]
pub fn run_cli_wrapper_manifest(
    manifest_root: &toml::Value,
    run: &RunArgs,
    transparency_log: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    spirit_host: Option<Arc<dyn maos_host::SpiritHostPort>>,
    enterprise_runtime: Option<Arc<enterprise_identity::EnterpriseRuntime>>,
    enterprise_pdp_runtime: Option<&enterprise_pdp_runtime::EnterprisePdpRuntime>,
    delegated_task: Option<&str>,
    remote_requested: bool,
    supervision: &dyn crate::supervision::WorkerSupervision,
    task: crate::supervision::WorkerTask,
    #[cfg(unix)] egress: Option<&crate::egress::EgressHandle>,
) -> Result<worker_cli::WorkerCompletion, Box<dyn std::error::Error>> {
    use maos_domain::host_grant::HostGrantAllowlist;
    use maos_domain::invariants::i9::SandboxTier;
    use maos_kernel_core::lifecycle::cli_wrapper::{
        admit_cli_wrapper_journaled, argv_prefix_hash, reject_respawn_with_context,
        resolve_cli_wrapper_tier, spawn_and_bridge, Backpressure, BridgeSpawnSpec,
    };
    use maos_kernel_core::security::manifest::CliWrapperConfig;
    // Parse and resolve resources before probes, host binding, or token minting.
    let resolved_caps = resolve_worker_caps(manifest_root, policy.as_ref())?;

    // 1. Parse [cli_wrapper].
    let cw_toml = toml::to_string(
        manifest_root
            .get("cli_wrapper")
            .ok_or("maos run: missing [cli_wrapper] section")?,
    )
    .map_err(|e| format!("maos run: serialize [cli_wrapper]: {e}"))?;
    let mut config = CliWrapperConfig::from_toml_str(&cw_toml)
        .map_err(|e| format!("maos run: [cli_wrapper] parse: {e}"))?;

    // 1b. Story 16-3 (D-16-3-D) — `[on_crash]` (FR50) and `[supervision]`
    //     (the ProgressWatchdog's threshold) are parsed HERE, at pid 0, with
    //     every other admission gate and BEFORE any load: a malformed or
    //     out-of-bounds section REFUSES the run rather than being clamped to a
    //     default. The kernel's own loader (`lifecycle/upgrade.rs`) is
    //     `pub(crate)`, so this repeats its extract-then-parse shape rather
    //     than calling it.
    //
    //     Absent sections are not an error: `None` means the kernel's
    //     documented defaults (`Nack`, 30 000 ms — the NFR's ">30s").
    let on_crash_section = match manifest_root.get("on_crash") {
        Some(value) => {
            let text = toml::to_string(value)
                .map_err(|e| format!("maos run: serialize [on_crash]: {e}"))?;
            Some(
                maos_kernel_core::security::manifest::OnCrashSection::from_toml_str(&text)
                    .map_err(|e| format!("maos run: [on_crash] parse: {e}"))?,
            )
        }
        None => None,
    };
    let supervision_section = match manifest_root.get("supervision") {
        Some(value) => {
            let text = toml::to_string(value)
                .map_err(|e| format!("maos run: serialize [supervision]: {e}"))?;
            Some(
                maos_kernel_core::security::manifest::SupervisionSection::from_toml_str(&text)
                    .map_err(|e| format!("maos run: [supervision] parse: {e}"))?,
            )
        }
        None => None,
    };

    // 2. Requested sandbox tier (defaults to the T3 CliWrapper floor).
    let requested_tier = match manifest_root
        .get("sandbox")
        .and_then(|s| s.get("tier"))
        .and_then(|t| t.as_str())
    {
        Some(s) => parse_sandbox_tier(s)?,
        None => SandboxTier::T3,
    };

    // 3. AC1 FORK C — fail loud at load on the deferred respawn_with_context.
    reject_respawn_with_context(&config).map_err(|e| format!("maos run: {e}"))?;
    if requested_tier == SandboxTier::T3 {
        maos_kernel_core::lifecycle::cli_wrapper::admission::reject_t3_stdin_posture(&config)?;
    }

    // 4. AC5 FORK A — host-grant tier gate (T3: self-grant KILLED). The manifest
    //    supplies only the REQUEST (its claimed image + author); the allowlist is
    //    HOST-MANAGED (built-in hermetic fixture/probe grants + operator
    //    `MAOS_HOST_GRANTS` file), never populated from manifest fields.
    //    A (image, author) that matches no host grant fails CLOSED at
    //    `resolve_cli_wrapper_tier` (ECliWrapperTierNotGranted) — the artifact
    //    can no longer decide its own tier.
    let attested_image = config.command.clone();
    let signing_key_id = manifest_root
        .get("author")
        .and_then(|a| a.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();
    let allowlist = load_host_grant_allowlist();
    let granted_tier =
        resolve_cli_wrapper_tier(requested_tier, &attested_image, &signing_key_id, &allowlist)
            .map_err(|e| format!("maos run: {e}"))?;

    // The host grant bounds admission; the proxy enforces exact destinations
    // after the verified container launch. Emit the disposition only after its
    // confirmed spawn report, never for a refused or failed launch.
    let permitted_egress = allowlist
        .lookup(&attested_image, &signing_key_id)
        .map(|g| g.permitted_egress_destinations.as_slice())
        .unwrap_or_default();

    // 5. Resolve the CLI binary path; pin it into the config for the probe.
    let resolved = resolve_cli_binary(&config.command)?;
    config.command = resolved.clone();

    // Story 11.1a AC1 — invoke the SpiritHostPort at the exact point the
    // kernel's BridgeSpawnSpec.program is computed. `[cli_wrapper]` manifests
    // declare no authoring form field (schema extension is out of 11.1a's
    // scope — see story "Explicitly NOT in 11.1a"), so every request here is
    // `NativeSubprocess`; when `spirit_host` is `Some`, this is a REAL,
    // non-inert call (identity resolution) rather than dead code. Absent a
    // port (native-only default build), `resolved` is used directly —
    // byte-identical to pre-11.1a behavior.
    let resolved = match &spirit_host {
        Some(host) => {
            host.resolve_launch(&maos_host::SpiritLaunchRequest {
                form: maos_host::SpiritForm::NativeSubprocess,
                artifact: resolved,
                form_config: vec![],
            })
            .map_err(|e| format!("maos run: spirit host resolve_launch: {e}"))?
            .program
        }
        None => resolved,
    };

    // T2/T1 — select the swappable Worker-CLI adapter by resolved binary and
    // route the typed task into its argv. An unsupported wrapper fails CLOSED
    // here, before the admission probe or any spawn (spec: "refuse before
    // probe/spawn"; "fail closed on ... unsupported wrappers").
    let worker_cli = worker_cli::select_worker_cli(&resolved).ok_or_else(|| {
        format!(
            "maos run: unsupported cli_wrapper command '{resolved}'; supported worker CLIs: {:?}",
            worker_cli::SUPPORTED_WORKER_CLIS
        )
    })?;
    // T5 — CI/local split: real agent CLIs need MAOS_LIVE_AGENT (local opt-in);
    // hermetic fixtures always run. CI never sets the flag, so no paid agent
    // spawns on the hermetic path. Fails closed before spawn.
    let live_agent = std::env::var_os("MAOS_LIVE_AGENT").is_some_and(|v| !v.is_empty());
    worker_cli::live_agent_gate(worker_cli.name(), live_agent)
        .map_err(|e| format!("maos run: {e}"))?;
    // T5 — clean-home invariant on the live path: refuse the CLI's ambient auth
    // file. Child environments are cleared; HOME is explicitly restored from
    // this runtime environment, so it determines the credential-file lookup.
    //
    // j1-crosshost-2a AC2.4 — an UNSET `HOME` used to skip the check entirely,
    // which is a fail-OPEN: the CLI's own home resolution does not necessarily
    // give up when the variable is missing, so "we could not look" was silently
    // treated as "there is nothing there". On the live path an unverifiable
    // clean-home invariant is a REFUSAL. The hermetic path is untouched — this
    // whole block is `live_agent`-gated and CI never sets that flag.
    if live_agent {
        // Review 2a-P7 — EMPTY is as unverifiable as UNSET: an empty `HOME`
        // yields RELATIVE `.codex/auth.json` paths that scan the daemon's cwd
        // while the child may apply its own empty-HOME fallback, so the
        // invariant would read "satisfied" while proving nothing.
        let home = match std::env::var_os("HOME") {
            Some(h) if !h.is_empty() => h,
            _ => {
                return Err(
                    "maos run: HOME is unset or empty on the live path, so the clean-home \
                            invariant cannot be checked — refusing. An unverifiable credential \
                            control is not a satisfied one: set HOME to the run's sandbox home."
                        .into(),
                );
            }
        };
        if let Err(p) =
            worker_cli::refuse_ambient_auth(worker_cli.as_ref(), std::path::Path::new(&home))
        {
            return Err(format!(
                "maos run: ambient auth file {} present in the sandbox home — refusing the \
                 live run. It lets the worker use a credential MAOS never holds, so redaction \
                 is unattestable (a failed Tier-2). Wipe it or use a clean sandbox home. Note \
                 this is a FILENAME control: renaming the file satisfies it without removing \
                 the credential. Provider keys are not copied into the child environment.",
                p.display()
            )
            .into());
        }
    }
    // j1-crosshost-1a AC1.6 / AC3.5 — the task is FRAME-BORNE. It arrives as a
    // parameter drained from the delegation `task.assign` payload; the
    // `MAOS_WORKER_TASK` env read and its `DEFAULT_WORKER_TASK` fallback are
    // DELETED, not bypassed.
    //
    // `None` is the absence of a delegation, NOT a default: the standalone
    // `[cli_wrapper]` path has no delegating Orchestrator, so the worker runs with
    // no trailing task argv (the fixture falls back to its canned first line —
    // see `spirits/worker/src/bin/worker-cli-fixture.rs`). Reintroducing a default
    // string here would recreate exactly the shortcut this story removed.
    let task_args = match delegated_task {
        Some(task) => worker_cli.argv(task),
        None => Vec::new(),
    };
    let probe_home = if worker_cli.name() == worker_cli::PROBE_CLI_NAME {
        Some(
            maos_domain::operator_door::maos_home()
                .map_err(|error| format!("maos run: cannot resolve MAOS home: {error}"))?
                .ok_or_else(|| {
                    "maos run: the worker probe requires a resolved MAOS_HOME for its control-file check"
                        .to_string()
                })?,
        )
    } else {
        None
    };
    let mut worker_env = worker_cli.nonsecret_env();
    if let Some(home) = probe_home.as_deref() {
        worker_env.extend(worker_cli.control_path_env(home));
    }
    let mut proxy_args = Vec::new();

    // 6. Admission — adapter-aware. The hermetic fixture speaks the kernel's
    //    Story 7.4 `--maos-bridge-probe` output-shape handshake, so it uses the
    //    journaled kernel path (probe + shape assert + T3 floor). A real
    //    adapter-backed CLI (codex/claude) does NOT implement that handshake —
    //    it exits non-zero on the probe flag, which is why the fixture was the
    //    only worker that ever admitted. For real CLIs the WorkerCli adapter's
    //    `parse_completion` IS the output-shape contract (verified at COMPLETION),
    //    so admission runs a liveness probe and re-asserts the T3 floor here.
    #[cfg(unix)]
    let proxy_configured = egress.is_some_and(crate::egress::EgressHandle::is_available);
    #[cfg(not(unix))]
    let proxy_configured = false;
    match worker_cli.probe_strategy() {
        worker_cli::ProbeStrategy::BridgeHandshake => {
            admit_cli_wrapper_journaled(
                &config,
                granted_tier,
                0,
                &transparency_log,
                proxy_configured,
                &permitted_egress,
            )
            .map_err(|e| format!("maos run: cli_wrapper admission failed: {e}"))?;
        }
        worker_cli::ProbeStrategy::Liveness { argv } => {
            maos_kernel_core::lifecycle::cli_wrapper::admission::check_egress(
                &config,
                proxy_configured,
                &permitted_egress,
            )
            .map_err(|e| format!("maos run: cli_wrapper admission failed: {e}"))?;
            // AC6 floor — a CliWrapperSpirit requires T3 (the kernel probe asserts
            // this; preserve it on the real-CLI path, typed — Story 17-6 D-17-6-C).
            if !matches!(granted_tier, SandboxTier::T3) {
                let e = maos_domain::cli_wrapper::CliWrapperAdmissionError::ECliWrapperRequiresT3 {
                    observed_tier: format!("{granted_tier:?}"),
                };
                return Err(format!("maos run: cli_wrapper admission failed: {e}").into());
            }
            worker_cli::run_liveness_probe(&resolved, &argv, std::time::Duration::from_secs(10))
                .map_err(|e| {
                    format!("maos run: cli_wrapper admission failed: liveness probe: {e}")
                })?;
            eprintln!(
                "maos run: cli_wrapper '{}' admitted via liveness probe (real adapter-backed CLI; \
                 output shape is verified at completion by the {} adapter, not a bridge handshake)",
                resolved,
                worker_cli.name()
            );
        }
    }

    // 7. Issue the Scope::CliSubprocessSpawn cap-token (binds argv_prefix_hash).
    //    Mediation requires the operator policy to grant `proc.exec` for the CLI
    //    binary — the operator-facing capability grant is Epic 9 surface (FORK B /
    //    Cross-Impact #2). When the policy has not (yet) granted it, the spawn
    //    proceeds under the AC5 host-grant authority (attested-image + tier grant,
    //    a STRONGER operator authorization than the cap-token policy) with a LOUD
    //    audit note — never a silent bypass. The CapabilityInvocation exit row is
    //    journaled regardless. The full cap-token issue→bind→revoke lifecycle is
    //    proven in `maos-capability::cap_tokens::tests::cli_subprocess_exit_revoke`.
    // Story 11.4c AC2 — SSO/PDP governance happens at the composition root,
    // before the kernel's frozen `CapabilityToken` is minted. Enterprise SSO
    // verifies `MAOS_SSO_ASSERTION`; Enterprise PDP receives the verified
    // principal attributes; only then does the kernel issue the token.
    // AC1.3 — the adapter's completion oracle depends on argv flags the adapter
    // itself cannot see: flags live only in the manifest and are hashed into the
    // cap-token. Refuse HERE, where `config.argv_prefix` is in scope and before
    // the hash is bound and the child is spawned, so a manifest that ships prose
    // to a JSON oracle fails loud instead of journaling a real success as a
    // non-completion (F4's inversion). For claude this also enforces `--bare`,
    // which is a REPRODUCIBILITY precondition, not only credential hygiene (F21).
    worker_cli::refuse_missing_argv_flags(worker_cli.as_ref(), &config.argv_prefix)
        .map_err(|e| format!("maos run: {e}"))?;
    // Review 2a-P2 — a bypass or repeated isolation flag makes the HASHED
    // posture a lie the sealed capture would then repeat: adapters re-parse
    // repeated flags last-wins, so the run can execute without the jail the
    // manifest declares, and claude's bypass modes also suppress the
    // `permission_denials` signal the oracle's verdict rests on. Same seam,
    // still before the hash is bound and the child spawned.
    worker_cli::refuse_unsafe_argv(worker_cli.as_ref(), &config.argv_prefix)
        .map_err(|e| format!("maos run: {e}"))?;
    // T3 launch requires a production trust anchor and verified image
    // attestation before binding or minting; there is no development fallback.
    let trust_anchor =
        maos_kernel_core::security::sandbox::t3::image_verify::read_trust_anchor_pub()
            .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;
    let crypto = maos_kernel_core::security::crypto::RingCryptoProvider;
    let lock = maos_kernel_core::security::sandbox::t3::image_lock::load_and_verify_lock(
        &trust_anchor,
        &crypto,
    )
    .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;
    let image = lock
        .default_entry()
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)?;
    let aph = argv_prefix_hash(&config.argv_prefix);

    // 7a. Story 16-3 (D-16-3-D) — BIND. Every gate above ran at pid 0 and is
    //     unchanged: a refused manifest must never leave `lifecycle.load` rows
    //     behind. From here on the Worker HAS an SCB, and the guard's `Drop`
    //     owns every exit path.
    #[cfg(unix)]
    let egress_policy = egress.map(|handle| {
        let ceiling_ms = supervision_section.as_ref().map_or_else(
            || maos_manifest::SupervisionSection::default().silent_failure_threshold_ms,
            |section| section.silent_failure_threshold_ms,
        );
        (
            handle,
            std::time::Duration::from_millis(u64::from(ceiling_ms))
                .min(std::time::Duration::from_secs(300)),
        )
    });
    let bundle = maos_kernel_core::scheduler::control_block::SpiritManifestBundle {
        on_crash: on_crash_section,
        supervision: supervision_section,
        ..Default::default()
    };
    let binding = supervision.bind(&task, bundle).map_err(|e| e.to_string())?;
    let spirit_pid = binding.spirit_pid();
    let worker_spirit_id = binding.spirit_id().to_string();
    let mut egress_socket = None;
    #[cfg(unix)]
    let binding = {
        let mut binding = binding;
        if let Some((handle, ttl)) = egress_policy {
            let credential = if config.egress.is_empty() {
                maos_egress::Credential::None
            } else if let Some(name) = worker_cli.credential_env_var() {
                // Do not format VarError: its non-Unicode variant owns the key.
                let raw = std::env::var(name)
                    .map_err(|_| format!("maos run: egress credential {name} is unavailable"))?;
                if worker_cli.proxy_uses_api_key() {
                    maos_egress::Credential::ApiKey(raw)
                } else {
                    maos_egress::Credential::Bearer(raw)
                }
            } else {
                maos_egress::Credential::None
            };
            let lease = handle
                .spawn(
                    spirit_pid,
                    &worker_spirit_id,
                    &config.egress,
                    &permitted_egress,
                    credential,
                    ttl,
                )
                .map_err(|error| format!("maos run: Worker egress: {error}"))?;
            egress_socket = Some(lease.socket_path().to_owned());
            let proxy_root = "http://127.0.0.1:18080";
            worker_env.extend(worker_cli.proxy_env(proxy_root, lease.bearer()));
            proxy_args.extend(worker_cli.proxy_args(proxy_root));
            binding.attach_egress(lease);
        }
        binding
    };
    #[cfg(not(unix))]
    let binding = binding;

    // 7b. The mint moves to the REAL pid. Its revoke row, its
    //     `identity.asserted` row under an enterprise posture, and the token
    //     the in-flight record carries all belong to the Worker, not to pid 0.
    let token_id = match issue_enterprise_governed_capability(
        capability.as_ref(),
        enterprise_runtime.as_deref(),
        enterprise_pdp_runtime,
        spirit_pid,
        Scope::CliSubprocessSpawn {
            cli_binary_path: resolved.clone(),
            argv_prefix_hash: aph,
            output_shape_version: config.output_shape_version.clone(),
        },
        300,
        [0u8; 32],
        IntentClass::Standard,
    ) {
        Ok(t) => Some(t.token_id),
        Err(e) => {
            // §A6 review P16 (decision D1, ratified 2026-08-17). A GOVERNANCE
            // DENIAL — PDP `Deny`, or SSO configured while MAOS_SSO_ASSERTION
            // is absent — is a REFUSAL on the REMOTE-requested path (host B
            // serving a frame another Host sent): 2a's FORK B let the LOCAL
            // `maos run` path proceed under AC5 host-grant authority with a
            // loud note, but carrying that posture onto the receiving side
            // would make host B the weaker endpoint exactly at the trust
            // boundary the enterprise threading exists to guard. The inbound
            // frame is already journaled (I2), so the refusal is durable
            // evidence, not a silent drop. A `KernelPolicyDenied`/`Mediation`
            // failure — the kernel `proc.exec` policy grant not existing yet —
            // keeps 2a's posture on BOTH paths: the AC5 host grant is the
            // ratified stronger authority there (Epic 9 surface), and hermetic
            // deployments legitimately run without a policy table.
            //
            // Story 16-3: this refusal is now AFTER `bind`, so it leaves
            // through the guard — `lifecycle.unload`, a PlannedUnload receipt
            // and the stop disposition, never a bound Worker with no process.
            //
            // Story 17-9 (D-17-9-FW-06 A): D1's proceed posture is for the
            // ABSENT `proc.exec` grant only. `QuotaExhausted` means the kernel
            // evaluated and allowed the scope but this Worker's issuance window
            // is spent — proceeding would make exhaustion a tokenless fallback,
            // so it refuses on BOTH paths, through the same guard.
            match &e {
                GovernedMintError::QuotaExhausted | GovernedMintError::ContextExhausted => {
                    return Err(format!(
                        "maos run: cli_wrapper spawn REFUSED: {e}; a quota refusal never \
                         proceeds without a kernel token (Story 17-9 FW-06)"
                    )
                    .into());
                }
                GovernedMintError::SsoAssertionMissing(_) | GovernedMintError::PdpDenied(_)
                    if remote_requested =>
                {
                    return Err(format!(
                        "host B REFUSES the remote-requested spawn: governance denial ({e}). \
                         On the receiving path a PDP/SSO denial is a refusal, never a \
                         downgrade to host-grant authority (§A6 review D1)"
                    )
                    .into());
                }
                GovernedMintError::SsoAssertionMissing(_)
                | GovernedMintError::PdpDenied(_)
                | GovernedMintError::KernelPolicyDenied
                | GovernedMintError::Mediation(_) => {}
            }
            eprintln!(
                "maos run: cli_wrapper cap-token mediation not granted ({e}); proceeding under \
                 AC5 host-grant authority (operator policy `proc.exec` grant is Epic 9 surface). \
                 The CapabilityInvocation exit row is still journaled."
            );
            None
        }
    };
    // The record was pushed at bind with a zero token and no deadline; patch it
    // now that the real one exists. A refused mediation leaves `TokenId([0;16])`
    // — stated as "no kernel token", never an invented one.
    if let Some(tid) = token_id {
        let ttl_deadline_ns = maos_kernel_core::capability::cap_tokens::monotonic_now_ns()
            .saturating_add(300u64.saturating_mul(1_000_000_000));
        supervision.bind_token(binding.core(), tid, ttl_deadline_ns);
    }

    // 7c. A stop that landed DURING the mint must not spawn a child. The phase
    //     and the supervisor-wide latch are read here (lock → decide → release,
    //     inside the port), and a refusal leaves through the guard.
    supervision.after_mint();
    if supervision.is_stopping()
        || binding.core().phase() != crate::supervision::BindingPhase::Running
    {
        return Err(
            crate::supervision::WorkerSupervisionError::StoppedBeforeSpawn
                .to_string()
                .into(),
        );
    }

    let boot_nonce = maos_kernel_core::capability::cap_tokens::monotonic_now_ns();
    let container_name = format!("maos-{worker_spirit_id}-{boot_nonce}");
    let state_dir = supervision
        .create_t3_state_dir()
        .map_err(crate::supervision::WorkerSupervisionError::Start)?;
    let payload_pidfd = supervision.payload_pidfd();
    let core = Arc::downgrade(binding.core());
    let launch_custody = maos_kernel_core::lifecycle::cli_wrapper::ContainerLaunchCustody::new(
        state_dir.clone(),
        move |terminal| {
            if let Some(core) = core.upgrade() {
                core.attach_container(terminal);
            }
        },
    );
    binding
        .core()
        .attach_launch_custody(Arc::clone(&launch_custody));
    proxy_args.extend(task_args);
    let spec = BridgeSpawnSpec {
        program: resolved,
        sandbox: maos_kernel_core::security::sandbox::SandboxSpec {
            tier: granted_tier,
            resolved_caps,
            declared_scopes: vec![],
            spirit_id: worker_spirit_id.clone(),
            output_shape_predicate: None,
        },
        argv_prefix: config.argv_prefix.clone(),
        task_args: proxy_args,
        expected_argv_prefix_hash: aph,
        from_spirit_id: worker_spirit_id.clone(),
        stdio_shape: config.posture.stdio_shape,
        control_channel: config.posture.control_channel,
        shutdown_signal: config.posture.shutdown_signal.clone(),
        channel_capacity: 256,
        backpressure: Backpressure::Block,
        env: worker_env,
        t3: Some(maos_kernel_core::lifecycle::cli_wrapper::T3Launch {
            image,
            boot_nonce,
            container_name,
            egress_socket,
            state_dir: state_dir.clone(),
            payload_pidfd,
            launch_custody: Some(Arc::clone(&launch_custody)),
        }),
    };
    // Bound the completion read-back to THIS run's journaled CliSubprocessOutput
    // rows (the adapter's oracle reads the persisted evidence, not the raw exit).
    let worker_run_since_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let bridge_result = spawn_and_bridge(spec);
    if bridge_result.is_err() {
        launch_custody.remove_state_dir_if_unowned();
    }
    let bridge = bridge_result?;
    // Story 16-3 (D-16-3-Q (6)) — guard FIRST. `SpawnedBridge::drop` kills and
    // reaps the child, and struct fields drop in declaration order, so two
    // separate locals would let the bridge kill the child before the guard
    // acted: the guard's signal would prove nothing, and the bridge's SIGKILL
    // landing in `Running` could race the observer into a spurious crash path.
    let mut live = crate::supervision::LiveWorker { binding, bridge };
    let (child, image) = live.bridge.container().ok_or_else(|| {
        maos_kernel_core::lifecycle::cli_wrapper::BridgeError::SandboxRefused(
            "Worker spawn did not produce a T3 container".into(),
        )
    })?;
    let terminal = live.bridge.container_terminal().ok_or_else(|| {
        maos_kernel_core::lifecycle::cli_wrapper::BridgeError::SandboxRefused(
            "Worker spawn did not produce a T3 terminal handle".into(),
        )
    })?;
    let container_id = terminal.container_id().to_owned();
    live.binding.core().attach_container(terminal);
    supervision.commit_container_spawn(live.binding.core(), child, image)?;
    println!(
        "{}",
        serde_json::json!({
            "event": "host_grant_disposition",
            "attested_image": attested_image,
            "signing_key_id": signing_key_id,
            "granted_tier": format!("{granted_tier:?}"),
            "egress": "enforced",
            "egress_enforced": true,
            "permitted_egress": permitted_egress,
        })
    );
    let child_pid = live.bridge.child_pid();
    println!(
        "{}",
        serde_json::json!({
            "event": "cli_wrapper_loaded",
            "spirit_id": worker_spirit_id,
            "spirit_pid": spirit_pid,
            "granted_tier": format!("{granted_tier:?}"),
            "child_pid": child_pid,
            "container_id": container_id,
            "live": run.live,
        })
    );

    // 8b. The exit observer — on THIS thread, BEFORE the pump. After the pump
    //     starts, the only signal available is EOF on both streams, which the
    //     last pipe holder controls, not the death.
    supervision.watch(live.binding.core(), child_pid);
    // AC3's injected-`?` point, pinned HERE: after the child handle is stored
    // and the observer is running, before the pump. A `?` at this position is
    // the honest test of the RAII guard — the guard must signal the live child,
    // disposition the record and unload the SCB with no explicit `abandon` call
    // anywhere on this path.
    supervision.error_after_watch()?;

    let pump = live.bridge.pump_to_journal(
        &transparency_log,
        spirit_pid,
        "kernel",
        &config.command,
        &["cli-wrapper-run".to_string()],
    );

    let cap_for_revoke = Arc::clone(&capability);
    // D-16-1-V — the revoke closure stays UNCONDITIONAL, at the Worker's real
    // pid: its `CliSubprocessExit` row is kept even when an unload already
    // revoked. The closure is `FnOnce` and is called exactly once.
    let exit = live.bridge.wait_and_finalize(
        &transparency_log,
        spirit_pid,
        Some(capability.audit_sender()),
        move |exit_code| {
            if let Some(tid) = token_id {
                let _ = cap_for_revoke.revoke_cli_subprocess_exit(tid, spirit_pid, exit_code);
            }
        },
    );

    // 8c. Both the completion oracle and classifier consume this finalized
    // bridge exit. T3 completion follows the payload's retained terminal facts:
    // only an available, voluntary payload exit (including exit 0) completes;
    // an intentional PlannedStop cannot complete even if the client exits 0.
    // T2/T1/native completion keeps its historical runtime-client exit mapping.
    // Eligibility is decided under the binding lock by `finish`, so an
    // unload/root stop racing this point can never yield a completion.
    let may_complete = live.binding.finish(&exit);
    let worker_exit = worker_exit_from_finalized_bridge(may_complete, &exit);

    println!(
        "{}",
        serde_json::json!({
            "event": "cli_wrapper_exit",
            "child_pid": child_pid,
            "stdout_lines": pump.stdout_lines,
            "stderr_lines": pump.stderr_lines,
            "exit_cause": format!("{:?}", exit.cause),
            "is_crash": exit.cause.is_crash(),
        })
    );
    eprintln!(
        "maos run: cli_wrapper '{}' exited ({:?}); {} CliSubprocessOutput row(s) journaled to the Transparency Log",
        config.command,
        exit.cause,
        pump.stdout_lines + pump.stderr_lines
    );
    let mut wc_stdout: Vec<String> = Vec::new();
    let mut wc_stderr: Vec<String> = Vec::new();
    // j1-crosshost-2a AC1.6 — NAMED for what it is. This is the last stdout
    // `CliSubprocessOutput` frame_id, assigned on EVERY stdout row and causally
    // unrelated to the oracle's verdict (which is computed below, at `:1281`).
    // The old name `completion_tl_ref` implied the oracle produced it, so the
    // demo's P4 beat conjoined two unrelated facts and the `tl_ref` half was a
    // null control that was true whenever the worker printed anything at all.
    //
    // It stays UNCONDITIONAL on purpose: the run you most need a citable TL
    // reference for is the one that FAILED. Gating emission on `Completed` would
    // delete the evidence pointer at exactly the moment someone asks what the
    // worker actually printed.
    let mut last_stdout_tl_ref: Option<[u8; 16]> = None;
    // Story 16-3 — the read-back filters on the Worker's OWN pid. Every Worker
    // of a root shares `from_spirit_id == "worker"` (the bridge's sender
    // identity), so with two concurrent Workers the old sender filter mixed
    // their output and each verdict was computed partly from the other's rows.
    match transparency_log.query_frames(FrameFilter {
        kind: Some(FrameKind::CliSubprocessOutput),
        spirit_pid: Some(spirit_pid),
        since_ns: Some(worker_run_since_ns),
        ..Default::default()
    }) {
        Ok(rows) => {
            for row in &rows {
                let Ok(v) = serde_json::from_slice::<serde_json::Value>(&row.payload_redacted)
                else {
                    continue;
                };
                let line = v
                    .get("line")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string();
                match v.get("stream").and_then(|s| s.as_str()) {
                    Some("stdout") => {
                        last_stdout_tl_ref = Some(row.frame_id);
                        wc_stdout.push(line);
                    }
                    Some("stderr") => wc_stderr.push(line),
                    _ => {}
                }
            }
        }
        Err(e) => {
            eprintln!("maos run: could not read worker output rows for completion: {e}")
        }
    }
    let completion = worker_cli.parse_completion(&wc_stdout, &wc_stderr, worker_exit);
    println!(
        "{}",
        serde_json::json!({
            "event": "worker_completion",
            "worker_cli": worker_cli.name(),
            "completion": completion.label(),
            "completed": completion.is_completed(),
            // Non-secret: the last stdout CliSubprocessOutput frame_id (hex) — the
            // Worker-produced TL reference a digest cites. `null` if none captured.
            // NOT a completion witness: see the declaration above.
            "last_stdout_tl_ref":
                last_stdout_tl_ref.map(|id| id.iter().map(|b| format!("{b:02x}")).collect::<String>()),
        })
    );

    // ── j1-crosshost-2b AC3.4(b) — JOURNAL THE VERDICT ───────────────────────
    //
    // `deferred-work.md` names this story as owner: *"the sealed capture's
    // completion claim cites `last_stdout_tl_ref` (documented in-code as NOT a
    // completion witness) and the oracle verdict itself is println-only, never
    // journaled"*. The `println!` above is the ONLY place the verdict existed, and
    // stdout is not evidence: it is not append-only, not redacted at insert, not
    // in the sealed bundle, and not queryable. So a signed capture could cite a
    // TL reference that is true (a stdout row exists) for a claim it does not
    // support (the oracle's decision), while the decision itself left no trace.
    //
    // This row is that trace. Deliberately narrow:
    //   * `TelemetryEvent` on an EXISTING `FrameKind` — a new kind would be
    //     write-only, which is the defect `journal_completion`'s doc already
    //     records (`maos-audit::kind_to_string` did not map `CliSubprocessOutput`,
    //     so most rows in the signed Tier-2 bundle rendered `unknown`).
    //   * `label()`, never a literal: the SIX-value vocabulary, so a
    //     `not_completed:permission_denied` is journaled as itself.
    //   * `last_stdout_tl_ref` is carried NEXT TO the verdict rather than as it,
    //     with the distinction written into the payload, so a reader joining the
    //     two cannot mistake one for the other again.
    //   * `insert_frame_event` mints its own kernel-side `frame_id` (no
    //     caller-supplied id), so this write cannot collide with a peer-supplied
    //     one and keeps its halt-on-duplicate semantics.
    //   * UNCONDITIONAL, exactly like `last_stdout_tl_ref`: the run you most need
    //     the verdict for is the one that FAILED.
    //   * Story 16-3: at the Worker's REAL pid, so `maos audit query --spirit
    //     worker` resolves it and the verdict sits in the same pid's row set as
    //     the output it judges.
    let verdict_payload = serde_json::json!({
        "event": "worker_completion_verdict",
        "worker_cli": worker_cli.name(),
        "spirit_id": worker_spirit_id,
        "verdict": completion.label(),
        "completed": completion.is_completed(),
        "last_stdout_tl_ref_is_not_a_completion_witness": true,
        "last_stdout_tl_ref":
            last_stdout_tl_ref.map(|id| id.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    });
    let _logged = transparency_log.insert_frame_event(
        FrameKind::TelemetryEvent,
        spirit_pid,
        None,
        "worker.completion-verdict",
        verdict_payload.to_string().as_bytes(),
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );

    // AC3.10 — return the typed oracle outcome. The topology caller emits
    // `TaskComplete` only for `WorkerCompletion::Completed`; a crash or missing
    // marker must leave the delegation in flight and fail the run.
    Ok(completion)
}

#[cfg(all(test, feature = "network"))]
mod resource_policy_tests {
    use super::resolve_worker_caps;
    use maos_kernel_core::capability::cap_policy::PolicyTable;
    use maos_manifest::{ResolvedCaps, ResourceCaps};
    use std::sync::Arc;

    fn set_resource_floor(policy: &PolicyTable, floor: ResourceCaps) {
        let mut snapshot = (*policy.inner().load_full()).clone();
        snapshot.operator_policy.resource_cap_floor = Some(floor);
        policy.inner().store(Arc::new(snapshot));
    }

    #[test]
    fn worker_caps_resolve_each_dimension_from_the_live_policy_snapshot() {
        let manifest: toml::Value = toml::from_str(
            "[resources]\ncpu_max_pct = 20\nmemory_max_mb = 256\nfd_max = 32\npids_max = 80\n",
        )
        .expect("parse resource manifest");
        let policy = PolicyTable::new();

        set_resource_floor(
            &policy,
            ResourceCaps {
                cpu_max_pct: Some(10),
                memory_max_mb: Some(512),
                fd_max: Some(64),
                pids_max: Some(40),
            },
        );

        assert_eq!(
            resolve_worker_caps(&manifest, &policy).expect("resolve current policy"),
            ResolvedCaps {
                cpu_max_pct: Some(10),
                memory_max_mb: Some(256),
                fd_max: Some(32),
                pids_max: Some(40),
            },
            "operator and manifest caps must win independently across all dimensions"
        );

        set_resource_floor(
            &policy,
            ResourceCaps {
                cpu_max_pct: Some(30),
                memory_max_mb: Some(128),
                fd_max: Some(16),
                pids_max: Some(100),
            },
        );
        assert_eq!(
            resolve_worker_caps(&manifest, &policy).expect("resolve updated policy"),
            ResolvedCaps {
                cpu_max_pct: Some(20),
                memory_max_mb: Some(128),
                fd_max: Some(16),
                pids_max: Some(80),
            },
            "each launch must read the latest policy snapshot rather than a frozen boot value"
        );
    }
}
