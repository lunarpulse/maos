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
}

#[cfg(feature = "network")]
/// Parse `run <manifest-path> [--live] [--once]` from the process args (the args
/// AFTER the binary name). Manual parsing — the binary has no clap dependency.
/// Returns `None` when the first arg is not `run` (the env-gated paths win).
pub fn parse_run_args<I: IntoIterator<Item = String>>(args: I) -> Result<Option<RunArgs>, String> {
    let mut it = args.into_iter();
    if it.next().as_deref() != Some("run") {
        return Ok(None);
    }
    let mut manifest_path: Option<String> = None;
    let mut live = false;
    let mut once = false;
    for a in it {
        match a.as_str() {
            "--live" => live = true,
            "--once" => once = true,
            // `--replay-llm` is the explicit hermetic flag JB-3's PTY command
            // uses; it is the DEFAULT (no `--live`) and accepted as a no-op so
            // the documented command string stays stable.
            "--replay-llm" => live = false,
            other if !other.starts_with("--") && manifest_path.is_none() => {
                manifest_path = Some(other.to_string());
            }
            other => {
                return Err(format!(
                    "maos run: unknown argument '{}' — expected: <manifest> [--live] [--once]",
                    other
                ));
            }
        }
    }
    match manifest_path {
        Some(manifest_path) => Ok(Some(RunArgs {
            manifest_path,
            live,
            once,
        })),
        None => Err(
            "maos run: missing manifest path — expected: maos run <manifest> [--live] [--once]"
                .into(),
        ),
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

/// Story 8.12 — resolve a CliWrapper `command` to a runnable path. The
/// deterministic fixture-CLI (`worker-cli-fixture`) is built as a sibling of the
/// daemon binary in the cargo target dir; tests run the daemon from
/// `target/debug/deps/`, so the parent dir is also checked, then `$PATH`.
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

/// Story 8.12 AC3 — load + run a `[cli_wrapper]` manifest under the daemon.
///
/// Admits the wrapper through the full gate stack — `reject_respawn_with_context`
/// (AC1 FORK C), `resolve_cli_wrapper_tier` against the host-side grant allowlist
/// (AC5 FORK A), then the existing journaled output-shape probe (Story 7.4
/// `admit_cli_wrapper_journaled`) — then issues a `Scope::CliSubprocessSpawn`
/// cap-token bound to the `argv_prefix_hash`, spawns the REAL subprocess through
/// the AC1 [`spawn_and_bridge`] bridge, journals each captured line as a
/// `FrameKind::CliSubprocessOutput=21` row, and on exit revokes the cap-token
/// with `RevokeReason::CliSubprocessExit`. Composition-root only — the kernel
/// receives a constructed handle and decides no topology.
/// Follow-up ID for the deferred packet-level egress enforcement. Run one records
/// egress `declared-not-enforced`; enforced egress is an Epic-14 v2.0 hardening.
#[cfg(feature = "network")]
const EGRESS_ENFORCEMENT_FOLLOWUP: &str = "FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT";

/// The built-in HOST grant for the hermetic fixture Worker. Host-side and
/// image-keyed — the host independently grants THIS known image; it never echoes
/// the manifest's self-declared fields (the AC5 trust-direction inversion).
#[cfg(feature = "network")]
pub fn builtin_fixture_grant() -> maos_domain::host_grant::HostGrant {
    maos_domain::host_grant::HostGrant {
        attested_image: "worker-cli-fixture".to_string(),
        signing_key_id: "MAOS Project".to_string(),
        permitted_tier: maos_domain::invariants::i9::SandboxTier::T3,
        permitted_egress_destinations: vec![], // hermetic: no egress
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
        let permitted_egress_destinations = g
            .get("permitted_egress_destinations")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        out.push(maos_domain::host_grant::HostGrant {
            attested_image,
            signing_key_id,
            permitted_tier,
            permitted_egress_destinations,
        });
    }
    Ok(out)
}

/// Load the HOST-MANAGED grant allowlist (replaces the v0.9 self-grant). The
/// built-in fixture grant keeps the hermetic path green; the operator adds real
/// agent-CLI grants (codex/claude + egress) via a `MAOS_HOST_GRANTS` TOML file.
/// A manifest whose (image, author) matches no grant fails CLOSED at
/// `resolve_cli_wrapper_tier` — never self-granted.
///
/// **Disposition of the unreadable/unparseable branch, stated rather than
/// inherited (j1-crosshost-2a AC2.4).** A `MAOS_HOST_GRANTS` file that cannot be
/// read or parsed does NOT abort the run: it warns and continues with the
/// built-in grants only. That is SAFE today — the built-in set grants the
/// hermetic fixture and nothing else, so every real agent CLI then fails closed
/// at `resolve_cli_wrapper_tier` and no unauthorized image can spawn. It is
/// nevertheless a SILENT DOWNGRADE OF AN OPERATOR-INTENT FILE: the operator
/// asked for a specific grant set and got a different one, and the only signal
/// is a line on stderr. It stays a warning because tightening it to a hard
/// refusal changes the failure mode of every hermetic run that happens to have a
/// stale variable exported, which is a separate decision from this story's. The
/// safety argument above is the reason it is tolerable, not an argument that it
/// is correct.
#[cfg(feature = "network")]
pub fn load_host_grant_allowlist() -> maos_domain::host_grant::StaticHostGrantAllowlist {
    let mut grants = vec![builtin_fixture_grant()];
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
    /// Kernel capability mediation failed (e.g. `proc.exec` not granted).
    Mediation(String),
}

#[cfg(feature = "network")]
impl std::fmt::Display for GovernedMintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
        .map_err(|e| {
            GovernedMintError::Mediation(format!("kernel capability mediation failed: {e}"))
        })?;

    if let (Some(runtime), Some(principal)) = (enterprise_runtime, principal.as_ref()) {
        runtime
            .persist_identity_asserted(spirit_pid, principal, &capability_key)
            .map_err(|e| GovernedMintError::Mediation(e.to_string()))?;
    }

    Ok(token)
}

/// j1-crosshost-1a AC1.6 — `DEFAULT_WORKER_TASK` and the `MAOS_WORKER_TASK` read
/// are DELETED. The Worker's task is frame-borne: `delegated_task` carries the
/// `goal` drained from the delegation `task.assign` payload, and `None` means
/// there was no delegation (the standalone `[cli_wrapper]` path), not a default.
///
/// Returns the adapter-parsed completion label so the caller can journal it as a
/// real `FrameKind::TaskComplete` frame (AC3.10).
#[cfg(feature = "network")]
#[allow(clippy::too_many_arguments)]
pub fn run_cli_wrapper_manifest(
    manifest_root: &toml::Value,
    run: &RunArgs,
    transparency_log: Arc<maos_kernel_core::iac::transparency_log::TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    spirit_host: Option<Arc<dyn maos_host::SpiritHostPort>>,
    enterprise_runtime: Option<Arc<enterprise_identity::EnterpriseRuntime>>,
    enterprise_pdp_runtime: Option<&enterprise_pdp_runtime::EnterprisePdpRuntime>,
    delegated_task: Option<&str>,
    remote_requested: bool,
) -> Result<worker_cli::WorkerCompletion, Box<dyn std::error::Error>> {
    use maos_domain::host_grant::HostGrantAllowlist;
    use maos_domain::invariants::i9::SandboxTier;
    use maos_kernel_core::lifecycle::cli_wrapper::{
        admit_cli_wrapper_journaled, argv_prefix_hash, reject_respawn_with_context,
        resolve_cli_wrapper_tier, spawn_and_bridge, Backpressure, BridgeSpawnSpec,
    };
    use maos_kernel_core::security::manifest::CliWrapperConfig;

    // 1. Parse [cli_wrapper].
    let cw_toml = toml::to_string(
        manifest_root
            .get("cli_wrapper")
            .ok_or("maos run: missing [cli_wrapper] section")?,
    )
    .map_err(|e| format!("maos run: serialize [cli_wrapper]: {e}"))?;
    let mut config = CliWrapperConfig::from_toml_str(&cw_toml)
        .map_err(|e| format!("maos run: [cli_wrapper] parse: {e}"))?;

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

    // 4. AC5 FORK A — host-grant tier gate (T3: self-grant KILLED). The manifest
    //    supplies only the REQUEST (its claimed image + author); the allowlist is
    //    HOST-MANAGED (built-in fixture grant + operator `MAOS_HOST_GRANTS` file),
    //    never populated from the manifest's own fields. A (image, author) that
    //    matches no host grant fails CLOSED at `resolve_cli_wrapper_tier`
    //    (ECliWrapperTierNotGranted) — the artifact can no longer decide its own
    //    tier. See host_grant.rs module doc + `load_host_grant_allowlist`.
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

    // Egress disposition (spec c3): run one records egress `declared-not-enforced`
    // with a follow-up ID; enforced packet-level egress is an Epic-14 v2.0
    // hardening. The permitted destinations come from the HOST grant, not the
    // manifest.
    let permitted_egress = allowlist
        .lookup(&attested_image, &signing_key_id)
        .map(|g| g.permitted_egress_destinations.clone())
        .unwrap_or_default();
    println!(
        "{}",
        serde_json::json!({
            "event": "host_grant_disposition",
            "attested_image": attested_image,
            "signing_key_id": signing_key_id,
            "granted_tier": format!("{granted_tier:?}"),
            "egress": "declared-not-enforced",
            "egress_enforced": false,
            "permitted_egress": permitted_egress,
            "egress_followup": EGRESS_ENFORCEMENT_FOLLOWUP,
        })
    );

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
    // T5 — CI/local split: a real agent CLI needs MAOS_LIVE_AGENT (local opt-in);
    // the fixture always runs. CI never sets the flag, so CI cannot spawn a paid
    // agent — the paid path is physically local-only. Fails closed before spawn.
    let live_agent = std::env::var_os("MAOS_LIVE_AGENT").is_some_and(|v| !v.is_empty());
    worker_cli::live_agent_gate(worker_cli.name(), live_agent)
        .map_err(|e| format!("maos run: {e}"))?;
    // T5 — clean-home invariant on the live path: refuse an ambient auth file
    // (codex's `~/.codex/auth.json`, claude's `~/.claude/.credentials.json`) that
    // would let the child use a credential MAOS never holds. `HOME` is read here,
    // not `MAOS_HOME` or `XDG_DATA_HOME`: the child inherits the real `HOME`
    // because the bridge does not clear the env, so this is the variable that
    // decides which credential file the CLI actually finds.
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
                 the credential, and the inherited environment channel is not covered at all.",
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
    // Non-secret CLI env only (e.g. codex CODEX_NON_INTERACTIVE). Credentials are
    // injected host-side on the live path, never in this argv/env shaping code.
    let worker_env = worker_cli.nonsecret_env();

    // 6. Admission — adapter-aware. The hermetic fixture speaks the kernel's
    //    Story 7.4 `--maos-bridge-probe` output-shape handshake, so it uses the
    //    journaled kernel path (probe + shape assert + T3 floor). A real
    //    adapter-backed CLI (codex/claude) does NOT implement that handshake —
    //    it exits non-zero on the probe flag, which is why the fixture was the
    //    only worker that ever admitted. For real CLIs the WorkerCli adapter's
    //    `parse_completion` IS the output-shape contract (verified at COMPLETION),
    //    so admission runs a liveness probe and re-asserts the T3 floor here.
    match worker_cli.probe_strategy() {
        worker_cli::ProbeStrategy::BridgeHandshake => {
            admit_cli_wrapper_journaled(&config, granted_tier, 0, &transparency_log)
                .map_err(|e| format!("maos run: cli_wrapper admission failed: {e}"))?;
        }
        worker_cli::ProbeStrategy::Liveness { argv } => {
            // AC6 floor — a CliWrapperSpirit requires T3 (the kernel probe asserts
            // this; preserve it on the real-CLI path).
            if !matches!(granted_tier, maos_domain::invariants::i9::SandboxTier::T3) {
                return Err(format!(
                    "maos run: cli_wrapper admission failed: {} requires SandboxTier::T3, \
                     host-granted {granted_tier:?}",
                    worker_cli.name()
                )
                .into());
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
    let aph = argv_prefix_hash(&config.argv_prefix);
    let token_id = match issue_enterprise_governed_capability(
        capability.as_ref(),
        enterprise_runtime.as_deref(),
        enterprise_pdp_runtime,
        0,
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
            // evidence, not a silent drop. A `Mediation` failure — the kernel
            // `proc.exec` policy grant not existing yet — keeps 2a's posture on
            // BOTH paths: the AC5 host grant is the ratified stronger authority
            // there (Epic 9 surface), and hermetic deployments legitimately run
            // without a policy table.
            if remote_requested {
                match &e {
                    GovernedMintError::SsoAssertionMissing(_) | GovernedMintError::PdpDenied(_) => {
                        return Err(format!(
                            "host B REFUSES the remote-requested spawn: governance denial ({e}). \
                             On the receiving path a PDP/SSO denial is a refusal, never a \
                             downgrade to host-grant authority (§A6 review D1)"
                        )
                        .into());
                    }
                    GovernedMintError::Mediation(_) => {}
                }
            }
            eprintln!(
                "maos run: cli_wrapper cap-token mediation not granted ({e}); proceeding under \
                 AC5 host-grant authority (operator policy `proc.exec` grant is Epic 9 surface). \
                 The CapabilityInvocation exit row is still journaled."
            );
            None
        }
    };

    // 8. Spawn the REAL bridge. The typed task is routed as the trailing argv
    //    (after the hashed argv_prefix); no probe flag → the worker runs its task.
    let spec = BridgeSpawnSpec {
        program: resolved,
        argv_prefix: config.argv_prefix.clone(),
        task_args,
        expected_argv_prefix_hash: aph,
        from_spirit_id: "worker".to_string(),
        stdio_shape: config.posture.stdio_shape,
        control_channel: config.posture.control_channel,
        shutdown_signal: config.posture.shutdown_signal.clone(),
        channel_capacity: 256,
        backpressure: Backpressure::Block,
        env: worker_env,
    };
    // Bound the completion read-back to THIS run's journaled CliSubprocessOutput
    // rows (the adapter's oracle reads the persisted evidence, not the raw exit).
    let worker_run_since_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut bridge = spawn_and_bridge(spec).map_err(|e| format!("maos run: bridge spawn: {e}"))?;
    let child_pid = bridge.child_pid();
    println!(
        "{}",
        serde_json::json!({
            "event": "cli_wrapper_loaded",
            "spirit_id": "worker",
            "granted_tier": format!("{granted_tier:?}"),
            "child_pid": child_pid,
            "live": run.live,
        })
    );

    let pump = bridge.pump_to_journal(
        &transparency_log,
        0,
        "kernel",
        &config.command,
        &["cli-wrapper-run".to_string()],
    );

    let cap_for_revoke = Arc::clone(&capability);
    let exit = bridge.wait_and_finalize(&transparency_log, 0, move |exit_code| {
        if let Some(tid) = token_id {
            let _ = cap_for_revoke.revoke_cli_subprocess_exit(tid, 0, exit_code);
        }
    });

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

    // T2/T1 — completion is decided by the adapter over the JOURNALED Worker
    // output (redacted at insert), never by the raw exit code. Reconstruct the
    // Worker's stdout/stderr from the CliSubprocessOutput rows this run wrote,
    // then let the adapter's per-CLI oracle rule. The last stdout frame is the
    // Worker-produced Transparency Log reference a digest would cite.
    let worker_exit = match exit.cause.exit_code() {
        Some(code) => worker_cli::WorkerExit::Exited(code),
        None => worker_cli::WorkerExit::Crashed,
    };
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
    match transparency_log.query_frames(FrameFilter {
        kind: Some(FrameKind::CliSubprocessOutput),
        since_ns: Some(worker_run_since_ns),
        ..Default::default()
    }) {
        Ok(rows) => {
            for row in &rows {
                if row.from_spirit_id != "worker" {
                    continue;
                }
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
    let verdict_payload = serde_json::json!({
        "event": "worker_completion_verdict",
        "worker_cli": worker_cli.name(),
        "verdict": completion.label(),
        "completed": completion.is_completed(),
        "last_stdout_tl_ref_is_not_a_completion_witness": true,
        "last_stdout_tl_ref":
            last_stdout_tl_ref.map(|id| id.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    });
    let _logged = transparency_log.insert_frame_event(
        FrameKind::TelemetryEvent,
        0,
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
