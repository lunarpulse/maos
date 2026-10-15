use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use maos_bin::supervision::{WorkerSupervision, WorkerSupervisor, WorkerTask};
use maos_domain::invariants::i1::{IntentClass, Scope};
use maos_kernel_core::capability::cap_policy::ManifestCapabilityScope;
use maos_kernel_core::iac::transparency_log::{FrameFilter, FrameKind, TransparencyLogAdapter};
use maos_kernel_core::lifecycle::cli_wrapper::runtime::{
    argv_prefix_hash, spawn_and_bridge, Backpressure, BridgeSpawnSpec, ExitCause,
};
use maos_kernel_core::scheduler::control_block::SpiritManifestBundle;
use maos_kernel_core::security::sandbox::SandboxSpec;
use maos_manifest::{CliWrapperConfig, OnCrashSection, SupervisionSection};

fn resolve_fixture(command: &str) -> Result<String, String> {
    let path = Path::new(command);
    if path.is_absolute() && path.is_file() {
        return Ok(command.to_owned());
    }
    let name = path
        .file_name()
        .ok_or_else(|| format!("invalid Worker command {command:?}"))?;
    if let Some(parent) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        for candidate in [
            parent.join(name),
            parent.parent().unwrap_or(&parent).join(name),
        ] {
            if candidate.is_file() {
                return Ok(candidate.display().to_string());
            }
        }
    }
    for dir in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate.display().to_string());
        }
    }
    Err(format!("could not resolve Worker fixture {command:?}"))
}

fn section_text(manifest: &toml::Value, key: &str) -> Result<Option<String>, String> {
    manifest
        .get(key)
        .map(|value| toml::to_string(value).map_err(|error| error.to_string()))
        .transpose()
}

pub fn run_t0_worker(
    manifest: &toml::Value,
    tl: Arc<TransparencyLogAdapter>,
    capability: Arc<maos_kernel_core::capability::CapabilityRegistryAdapter>,
    policy: Arc<maos_kernel_core::capability::cap_policy::PolicyTable>,
    supervisor: &WorkerSupervisor,
    task: WorkerTask,
) -> Result<maos_bin::worker_cli::WorkerCompletion, String> {
    let config_value = manifest
        .get("cli_wrapper")
        .ok_or_else(|| "manifest missing [cli_wrapper]".to_string())?;
    let config_text = toml::to_string(config_value).map_err(|e| e.to_string())?;
    if manifest
        .get("sandbox")
        .and_then(|sandbox| sandbox.get("tier"))
        .and_then(toml::Value::as_str)
        != Some("T0")
    {
        return Err("the in-process T0 primitive requires a sandbox tier of T0".to_string());
    }
    let config = CliWrapperConfig::from_toml_str(&config_text).map_err(|e| e.to_string())?;
    let program = resolve_fixture(&config.command)?;
    let on_crash = section_text(manifest, "on_crash")?
        .as_deref()
        .map(OnCrashSection::from_toml_str)
        .transpose()
        .map_err(|error| error.to_string())?;
    let supervision = section_text(manifest, "supervision")?
        .as_deref()
        .map(SupervisionSection::from_toml_str)
        .transpose()
        .map_err(|error| error.to_string())?;
    let bundle = SpiritManifestBundle {
        on_crash,
        supervision,
        ..Default::default()
    };
    let binding = supervisor.bind(&task, bundle).map_err(|e| e.to_string())?;
    let spirit_pid = binding.spirit_pid();
    let spirit_id = binding.spirit_id().to_owned();
    let aph = argv_prefix_hash(&config.argv_prefix);
    let scope = Scope::CliSubprocessSpawn {
        cli_binary_path: program.clone(),
        argv_prefix_hash: aph,
        output_shape_version: config.output_shape_version.clone(),
    };
    policy.inner().rcu(|old| {
        let mut next = (**old).clone();
        next.manifest_scopes.insert(
            spirit_pid,
            ManifestCapabilityScope {
                scopes: vec![scope.clone()],
                declared_tier: maos_domain::invariants::i9::SandboxTier::T0,
                ..Default::default()
            },
        );
        Arc::new(next)
    });
    let token = capability
        .issue_with_mediation(spirit_pid, scope, 300, [0; 32], IntentClass::Standard)
        .map_err(|error| error.to_string())?;
    let deadline = maos_kernel_core::capability::cap_tokens::monotonic_now_ns()
        .saturating_add(300_000_000_000);
    supervisor.bind_token(binding.core(), token.token_id, deadline);
    supervisor.after_mint();
    if supervisor.is_stopping()
        || binding.core().phase() != maos_bin::supervision::BindingPhase::Running
    {
        return Err(maos_bin::supervision::WorkerSupervisionError::StoppedBeforeSpawn.to_string());
    }

    let since_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let spec = BridgeSpawnSpec {
        program: program.clone(),
        sandbox: SandboxSpec {
            tier: maos_domain::invariants::i9::SandboxTier::T0,
            resolved_caps: Default::default(),
            declared_scopes: vec![],
            spirit_id: spirit_id.clone(),
            output_shape_predicate: None,
        },
        argv_prefix: config.argv_prefix.clone(),
        task_args: vec![],
        expected_argv_prefix_hash: aph,
        from_spirit_id: spirit_id.clone(),
        stdio_shape: config.posture.stdio_shape,
        control_channel: config.posture.control_channel,
        shutdown_signal: config.posture.shutdown_signal.clone(),
        channel_capacity: 256,
        backpressure: Backpressure::Block,
        env: vec![],
        t3: None,
    };
    let bridge = spawn_and_bridge(spec).map_err(|e| e.to_string())?;
    let mut live = maos_bin::supervision::LiveWorker { binding, bridge };
    let child_pid = live.bridge.child_pid();
    supervisor.watch(live.binding.core(), child_pid);
    supervisor.error_after_watch().map_err(|e| e.to_string())?;
    live.bridge.pump_to_journal(
        &tl,
        spirit_pid,
        "kernel",
        &config.command,
        &["cli-wrapper-run".to_string()],
    );
    let cap_for_revoke = Arc::clone(&capability);
    let token_id = token.token_id;
    let exit = live.bridge.wait_and_finalize(
        &tl,
        spirit_pid,
        Some(capability.audit_sender()),
        move |exit_code| {
            let _ = cap_for_revoke.revoke_cli_subprocess_exit(token_id, spirit_pid, exit_code);
        },
    );
    live.binding.finish(&exit);

    let worker_cli = maos_bin::worker_cli::select_worker_cli(&program)
        .ok_or_else(|| format!("unsupported Worker CLI {program:?}"))?;
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let rows = tl
        .query_frames(FrameFilter {
            kind: Some(FrameKind::CliSubprocessOutput),
            spirit_pid: Some(spirit_pid),
            since_ns: Some(since_ns),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
    let mut last_stdout_tl_ref = None;
    for row in rows {
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&row.payload_redacted) else {
            continue;
        };
        let line = value
            .get("line")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();
        match value.get("stream").and_then(|v| v.as_str()) {
            Some("stdout") => {
                last_stdout_tl_ref = Some(row.frame_id);
                stdout.push(line);
            }
            Some("stderr") => stderr.push(line),
            _ => {}
        }
    }
    let worker_exit = match exit.cause {
        ExitCause::Exited { code } => maos_bin::worker_cli::WorkerExit::Exited(code),
        _ => maos_bin::worker_cli::WorkerExit::Crashed,
    };
    let completion = worker_cli.parse_completion(&stdout, &stderr, worker_exit);
    let payload = serde_json::json!({
        "event": "worker_completion_verdict",
        "worker_cli": worker_cli.name(),
        "spirit_id": spirit_id,
        "verdict": completion.label(),
        "completed": completion.is_completed(),
        "last_stdout_tl_ref_is_not_a_completion_witness": true,
        "last_stdout_tl_ref": last_stdout_tl_ref.map(|id: [u8; 16]| id.iter().map(|b| format!("{b:02x}")).collect::<String>()),
    });
    let _ = tl.insert_frame_event(
        FrameKind::TelemetryEvent,
        spirit_pid,
        None,
        "worker.completion-verdict",
        payload.to_string().as_bytes(),
        maos_domain::invariants::i3::FrameOrigin::Kernel,
    );
    Ok(completion)
}
