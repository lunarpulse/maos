//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::{
    AuditFormat, AuditQuery, HaltArgs, HaltOp, ImportArgs, InstallArgs, OrchestratorArgs,
    OrchestratorOp, PauseArgs, PostureArgs, PostureChoice, ResolutionKindChoice, ResumeArgs,
    RevocationsArgs, RevocationsOp, RevokeTokenArgs, RunArgs, SpiritArgs, SpiritOp, Subcommand,
    UpgradePolicyArg,
};

pub fn dispatch(cmd: &Subcommand, color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(args) => install(args, color),
        Subcommand::Start(args) => lifecycle_verb("start", args.spirit.as_deref(), color),
        Subcommand::Stop(args) => lifecycle_verb("stop", args.spirit.as_deref(), color),
        Subcommand::Unload(args) => lifecycle_verb("unload", args.spirit.as_deref(), color),
        Subcommand::Uninstall(args) => lifecycle_verb("uninstall", args.spirit.as_deref(), color),
        Subcommand::Run(args) => run(args, color),
        Subcommand::Audit(args) => audit_dispatch(&args.query, color),
        Subcommand::Posture(args) => dispatch_posture(args, color),
        Subcommand::Halt(args) => dispatch_halt(args, color),
        Subcommand::Orchestrator(args) => dispatch_orchestrator(args, color),
        Subcommand::Pause(args) => dispatch_pause(args, color),
        Subcommand::Resume(args) => dispatch_resume(args, color),
        Subcommand::RevokeToken(args) => dispatch_revoke_token(args, color),
        Subcommand::Spirit(args) => dispatch_spirit(args, color),
        Subcommand::Revocations(args) => dispatch_revocations(args, color),
        Subcommand::Import(args) => dispatch_import(args, color),
    }
}

fn dispatch_import(args: &ImportArgs, _color: ColorChoice) -> ExitCode {
    // TODO: registry_uri is parsed by clap but not yet wired to storage location.
    // The LocalFsRegistryStorage uses a fixed default path; custom URI override
    // is deferred to v0.7+. Log a warning if the user provided one.
    if args.registry_uri.is_some() {
        eprintln!("maosctl import: warning: --registry-uri is not yet implemented ( Story 7.2 v1.0); ignored");
    }
    use maos_registry::admission::{admit_spirit, AdmissionConfig};
    use maos_registry::import;
    use maos_registry::origin::RegistryOrigin;
    use maos_registry::storage::{LocalFsRegistryStorage, RegistryStorage};
    use maos_registry::TrustTier;

    // 1. Extract bundle.
    let bundle = match import::extract_bundle(&args.offline) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl import: extract failed: {e}");
            return ExitCode::from(1);
        }
    };

    // 2. Verify per-file consistency.
    if let Err(e) = import::verify_bundle_consistency(&bundle) {
        eprintln!("maosctl import: bundle inconsistent: {e}");
        return ExitCode::from(1);
    }

    // 3. Build admission config.
    // Air-gapped imports default to Local tier unless --force-tier is used
    // and the operator policy allows it.
    let registry_origin_tier = if let Some(tier_str) = &args.force_tier {
        // Verify operator policy allows force-tier override.
        let policy_ok = std::env::var("MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT")
            .map(|v| v.to_lowercase() == "true" || v == "1")
            .unwrap_or(false);
        if !policy_ok {
            eprintln!("maosctl import: --force-tier requires MAOS_REGISTRY_ALLOW_FORCE_TIER_AT_IMPORT=true");
            return ExitCode::from(1);
        }
        match tier_str.as_str() {
            "local" => TrustTier::Local,
            "org_internal" => TrustTier::OrgInternal,
            "public_untrusted" => TrustTier::PublicUntrusted,
            other => {
                eprintln!("maosctl import: unrecognized tier '{other}'");
                return ExitCode::from(1);
            }
        }
    } else {
        TrustTier::Local // FR60: air-gapped imports are local-tier
    };

    let op_cfg = AdmissionConfig {
        tier_floor: TrustTier::Local,
        registry_origin_tier,
        t3_for_public_untrusted: false,
        allow_unsigned_local: true,
        org_signing_pubkey: None,
        runtime_provider_endpoint: None,
        runtime_crypto_provider: None,
    };

    // 4. Admit the Spirit.
    let decision = match admit_spirit(&bundle.signed_package, &op_cfg) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("maosctl import: admission failed: {e}");
            return ExitCode::from(1);
        }
    };

    // 5. Persist to local storage.
    if !args.dry_run {
        let storage = match LocalFsRegistryStorage::new() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("maosctl import: storage init failed: {e}");
                return ExitCode::from(1);
            }
        };
        let origin = RegistryOrigin::Imported {
            bundle_sha256: bundle.bundle_sha256.clone(),
        };
        if let Err(e) = storage.publish_with_origin(
            &bundle.signed_package.spirit_id,
            &bundle.signed_package.version,
            &bundle.signed_package,
            &origin,
        ) {
            eprintln!("maosctl import: persist failed: {e}");
            return ExitCode::from(1);
        }
    }

    let summary = serde_json::json!({
        "outcome": if args.dry_run { "dry_run" } else { "imported" },
        "spirit_id": bundle.signed_package.spirit_id.as_str(),
        "version": bundle.signed_package.version,
        "bundle_sha256": bundle.bundle_sha256,
        "manifest_bytes": bundle.signed_package.manifest_toml.len(),
        "artifact_bytes": bundle.signed_package.artifact_bytes.len(),
        "vetter_attestations": bundle.vetter_attestations.len(),
        "supplementary_claims": bundle.supplementary_claims.len(),
        "force_tier": args.force_tier,
        "registry_uri": args.registry_uri,
        "effective_tier": format!("{:?}", decision.effective_tier),
        "sandbox_tier_floor": format!("{:?}", decision.sandbox_tier_floor),
    });
    println!(
        "{}",
        serde_json::to_string(&summary).unwrap_or_else(|e| {
            eprintln!("maosctl import: failed to serialize summary: {e}");
            "{}".into()
        })
    );
    ExitCode::SUCCESS
}

fn run(args: &RunArgs, _color: ColorChoice) -> ExitCode {
    let spirit = match &args.spirit {
        Some(s) if s == "hello-spirit" => s,
        Some(s) => {
            eprintln!("maosctl: unknown spirit '{s}' — only 'hello-spirit' is available at v0.1-α");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("maosctl: run requires a spirit argument, e.g. 'maosctl run hello-spirit'");
            return ExitCode::from(2);
        }
    };

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", spirit);

    // Honor the accessibility cascade: pass NO_COLOR through if set
    if std::env::var_os("NO_COLOR").is_some() {
        cmd.env("NO_COLOR", "1");
    }
    // --plain flag also disables color
    if _color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn install(args: &InstallArgs, _color: ColorChoice) -> ExitCode {
    // At v0.1-α, install is a compilation check: build the hello-Spirit crate.
    let spirit_crate = match &args.source {
        Some(s) if s == "hello-spirit" => "maos-spirit-hello",
        Some(s) => {
            eprintln!("maosctl: unknown spirit '{s}' — only 'hello-spirit' is available at v0.1-α");
            return ExitCode::from(2);
        }
        None => {
            // Default: install hello-spirit (the only reference Spirit at v0.1)
            "maos-spirit-hello"
        }
    };

    // Decision Register D4 (Story 1b.5c): unit-test and integration-smoke
    // affordance. `MAOS_INSTALL_DRY_RUN=1` short-circuits the cargo build
    // so the accessibility cascade test in `tests/accessibility_test.rs`
    // can assert zero ANSI bytes without paying the ~30s build cost, and
    // the integration smokes (`maosctl_smoke.sh`, `v01_evaluator_path.sh`)
    // keep under 60s. The real cargo build path is exercised by the
    // release binary build step at the top of each integration script
    // (which compiles `maos-spirit-hello` transitively).
    if std::env::var_os("MAOS_INSTALL_DRY_RUN").is_some() {
        eprintln!("maosctl: {spirit_crate} compiled successfully");
        return ExitCode::SUCCESS;
    }

    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["build", "-p", spirit_crate, "--locked"]);

    match cmd.status() {
        Ok(s) if s.success() => {
            eprintln!("maosctl: {spirit_crate} compiled successfully");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("maosctl: cargo build {spirit_crate} failed");
            ExitCode::from(s.code().unwrap_or(2) as u8)
        }
        Err(e) => {
            eprintln!("maosctl: failed to execute cargo build: {e}");
            ExitCode::from(2)
        }
    }
}

/// v0.1-β lifecycle dispatch: shell out to `maos-bin` with
/// `MAOS_ONE_SHOT=<verb>` per Decision Register D2.
///
/// At v0.1-β each verb writes exactly one Lifecycle Journal entry and
/// exits. No supervisor, no mailbox, no `task.orphaned` emission — those
/// land in Epic 5 (Story 5.1) with a real supervised lifecycle. The
/// journal entry IS the observable v0.1 side-effect.
///
/// Decision Register D3: the shape is `spirit: Option<&str>` — all
/// three v0.1-β `*Args` structs are identical, but Epic 5 will
/// differentiate them (Stop will gain `--grace-period`, etc.), so the
/// distinct struct types in `cli.rs` are preserved.
///
/// Spirit-name validation is delegated to [`resolve_spirit_pid`]; the
/// returned `u32` is discarded — only the `Err(_)` branch is
/// load-bearing for lifecycle verbs (journal entries are keyed by
/// `spirit_id: String`, not `spirit_pid: u32`).
fn lifecycle_verb(verb: &str, spirit: Option<&str>, color: ColorChoice) -> ExitCode {
    let name = match spirit {
        Some(s) => s,
        None => {
            eprintln!(
                "maosctl: {verb} requires a spirit argument, e.g. 'maosctl {verb} hello-spirit'"
            );
            return ExitCode::from(2);
        }
    };

    if let Err(diag) = resolve_spirit_pid(name) {
        eprintln!("maosctl: {verb} — {diag}");
        return ExitCode::from(2);
    }

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", verb);
    cmd.env("MAOS_SPIRIT_ID", name);

    // Forward NO_COLOR / --plain through to the child for accessibility
    // (NFR-Ops-5). Mirror the existing `run` dispatch shape.
    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch_posture(args: &PostureArgs, color: ColorChoice) -> ExitCode {
    if let Err(diag) = resolve_spirit_pid(&args.spirit) {
        eprintln!("maosctl: posture — {diag}");
        return ExitCode::from(2);
    }

    let posture_env = match args.shift {
        PostureChoice::Cautious => "cautious",
        PostureChoice::Assistive => "assistive",
        PostureChoice::AutonomousWithHalt => "autonomous-with-halt",
    };

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", "posture-shift");
    cmd.env("MAOS_SPIRIT_ID", &args.spirit);
    cmd.env("MAOS_POSTURE", posture_env);

    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch_halt(args: &HaltArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        HaltOp::List { spirit, limit } => {
            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "halt-list");
            cmd.env("MAOS_HALT_LIMIT", limit.to_string());
            if let Some(s) = spirit {
                if let Err(diag) = resolve_spirit_pid(s) {
                    eprintln!("maosctl: halt list — {diag}");
                    return ExitCode::from(2);
                }
                cmd.env("MAOS_HALT_SPIRIT", s);
            }
            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }
            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
        HaltOp::Resolve {
            halt_id,
            spirit,
            kind,
            text,
            operator_policy,
        } => {
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maosctl: halt resolve — {diag}");
                return ExitCode::from(2);
            }
            // Defensive check (clap's required_if_eq handles most cases)
            match kind {
                ResolutionKindChoice::ProvidedContext if text.is_none() => {
                    eprintln!("maosctl: halt resolve --kind provided-context requires --text");
                    return ExitCode::from(2);
                }
                ResolutionKindChoice::AuthorizedOverride if operator_policy.is_none() => {
                    eprintln!("maosctl: halt resolve --kind authorized-override requires --operator-policy");
                    return ExitCode::from(2);
                }
                _ => {}
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "halt-resolve");
            cmd.env("MAOS_HALT_ID", halt_id);
            cmd.env("MAOS_HALT_SPIRIT", spirit);
            let kind_str = match kind {
                ResolutionKindChoice::ProvidedContext => "provided_context",
                ResolutionKindChoice::AcceptedHalt => "accepted_halt",
                ResolutionKindChoice::AuthorizedOverride => "authorized_override",
            };
            cmd.env("MAOS_HALT_KIND", kind_str);
            if let Some(t) = text {
                cmd.env("MAOS_HALT_TEXT", t);
            }
            if let Some(op) = operator_policy {
                cmd.env("MAOS_HALT_OPERATOR_POLICY", op);
            }
            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }
            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn dispatch_pause(args: &PauseArgs, color: ColorChoice) -> ExitCode {
    if let Err(diag) = resolve_spirit_pid(&args.spirit) {
        eprintln!("maosctl: pause — {diag}");
        return ExitCode::from(2);
    }

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", "pause");
    cmd.env("MAOS_SPIRIT_ID", &args.spirit);

    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch_resume(args: &ResumeArgs, color: ColorChoice) -> ExitCode {
    if let Err(diag) = resolve_spirit_pid(&args.spirit) {
        eprintln!("maosctl: resume — {diag}");
        return ExitCode::from(2);
    }

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", "resume");
    cmd.env("MAOS_SPIRIT_ID", &args.spirit);

    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch_orchestrator(args: &OrchestratorArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        OrchestratorOp::Queue {
            spirit,
            instruction,
        } => {
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maosctl: orchestrator queue — {diag}");
                return ExitCode::from(2);
            }
            if instruction.trim().is_empty() {
                eprintln!("maosctl: orchestrator queue — instruction must be non-empty");
                return ExitCode::from(2);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "orchestrator-queue");
            cmd.env("MAOS_ORCHESTRATOR_SPIRIT", spirit);
            cmd.env("MAOS_ORCHESTRATOR_INSTRUCTION", instruction);

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
        OrchestratorOp::Status { spirit } => {
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maosctl: orchestrator status — {diag}");
                return ExitCode::from(2);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "orchestrator-status");
            cmd.env("MAOS_ORCHESTRATOR_SPIRIT", spirit);

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn dispatch_revoke_token(args: &RevokeTokenArgs, color: ColorChoice) -> ExitCode {
    // Validate hex format BEFORE shelling out
    if args.token_id.len() != 32
        || !args
            .token_id
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
    {
        eprintln!(
            "maosctl: revoke-token — invalid token_id '{}' (expected 32-char lowercase hex)",
            args.token_id
        );
        return ExitCode::from(2);
    }

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", "revoke-token");
    cmd.env("MAOS_REVOKE_TOKEN_ID", &args.token_id);
    if let Some(ref reason) = args.reason {
        cmd.env("MAOS_REVOKE_REASON", reason);
    }

    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    match cmd.status() {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
}

fn dispatch_revocations(args: &RevocationsArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        RevocationsOp::Import { file, force } => {
            if !file.exists() {
                eprintln!(
                    "maosctl: revocations import — file not found: {}",
                    file.display()
                );
                return ExitCode::from(1);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "revocations-import");
            cmd.env("MAOS_CRL_PATH", file.as_os_str());
            if *force {
                cmd.env("MAOS_CRL_FORCE_REAPPLY", "1");
            }

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
        RevocationsOp::List => {
            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "revocations-list");

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
    }
}

fn dispatch_spirit(args: &SpiritArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        SpiritOp::HotSwapPrecheck { spirit, from, to } => {
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maosctl: spirit hot-swap-precheck — {diag}");
                return ExitCode::from(1);
            }
            if from.is_empty() {
                eprintln!("maosctl: spirit hot-swap-precheck — --from version must be non-empty");
                return ExitCode::from(2);
            }
            // Check the --to manifest path exists.
            let manifest_path = std::path::Path::new(to);
            if !manifest_path.exists() {
                eprintln!("maosctl: spirit hot-swap-precheck — manifest file not found: {to}");
                return ExitCode::from(1);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "hot-swap-precheck");
            cmd.env("MAOS_SPIRIT_ID", spirit);
            cmd.env("MAOS_HOTSWAP_FROM_VERSION", from);
            cmd.env("MAOS_HOTSWAP_TO_MANIFEST", to);

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
        SpiritOp::Upgrade { spirit, to, policy } => {
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maosctl: spirit upgrade — {diag}");
                return ExitCode::from(1);
            }
            let manifest_path = std::path::Path::new(to);
            if !manifest_path.exists() {
                eprintln!("maosctl: spirit upgrade — manifest file not found: {to}");
                return ExitCode::from(1);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "spirit-upgrade");
            cmd.env("MAOS_SPIRIT_ID", spirit);
            cmd.env("MAOS_UPGRADE_TO_MANIFEST", to);
            cmd.env(
                "MAOS_UPGRADE_POLICY",
                match policy {
                    UpgradePolicyArg::HotSwap => "hot-swap",
                    UpgradePolicyArg::ColdSwap => "cold-swap",
                    UpgradePolicyArg::Migrator => "migrator",
                },
            );

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
        SpiritOp::Inspect { spirit, sandbox } => {
            if !sandbox {
                eprintln!("maos: spirit inspect requires --sandbox at v0.3-β; full inspect surface arrives at Story 9.x");
                return ExitCode::SUCCESS;
            }
            if let Err(diag) = resolve_spirit_pid(spirit) {
                eprintln!("maos: {diag}");
                return ExitCode::from(2);
            }

            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "spirit-inspect");
            cmd.env("MAOS_SPIRIT_ID", spirit);
            cmd.env("MAOS_INSPECT_SANDBOX", "1");

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            match cmd.status() {
                Ok(s) if s.success() => ExitCode::SUCCESS,
                Ok(s) => ExitCode::from(s.code().unwrap_or(2) as u8),
                Err(e) => {
                    eprintln!(
                        "maosctl: failed to execute maos-bin at '{}': {e}",
                        bin.display()
                    );
                    ExitCode::from(2)
                }
            }
        }
    }
}

/// Resolve `maos-bin` binary path.
///
/// Priority: `MAOS_BIN_PATH` env var → sibling of current exe → PATH.
fn maos_bin_path() -> PathBuf {
    // 1. Explicit override
    if let Ok(p) = std::env::var("MAOS_BIN_PATH") {
        return PathBuf::from(p);
    }
    // 2. Sibling of current exe (same target directory)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("maos-bin");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    // 3. Fallback to PATH
    PathBuf::from("maos-bin")
}

fn audit_dispatch(query_kind: &Option<AuditQuery>, color: ColorChoice) -> ExitCode {
    match query_kind {
        // Bare `maosctl audit` — defaults to ndjson over all entries.
        None => audit_query(None, AuditFormat::Ndjson, color),
        Some(AuditQuery::Query { spirit, format }) => {
            audit_query(spirit.as_deref(), *format, color)
        }
    }
}

/// Resolve a Spirit name to its `spirit_pid` for filtering. At v0.1-β only
/// `hello-spirit` is resolvable (maps to `0` per Story 1b.5a's one-shot path).
/// Other names exit non-zero with a clear diagnostic — full Spirit registry
/// lookup is Epic 5.
fn resolve_spirit_pid(name: &str) -> Result<u32, String> {
    match name {
        "hello-spirit" => Ok(0),
        other => Err(format!(
            "unknown spirit, only 'hello-spirit' is available at v0.1-β (got '{other}')"
        )),
    }
}

fn audit_query(spirit: Option<&str>, format: AuditFormat, _color: ColorChoice) -> ExitCode {
    let db_path = default_transparency_log_path();

    let mut filter = maos_audit::AuditFilter::default();
    if let Some(name) = spirit {
        match resolve_spirit_pid(name) {
            Ok(pid) => filter.spirit_pid = Some(pid),
            Err(diag) => {
                eprintln!("maosctl: audit query — {diag}");
                return ExitCode::from(2);
            }
        }
    }

    let entries = match maos_audit::query(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit query — no Transparency Log found at {}. \
                 Run `maosctl run hello-spirit` first to seed the log.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit query — error: {e}");
            return ExitCode::from(2);
        }
    };

    let stdout = std::io::stdout();
    let lock = stdout.lock();
    // FR4 projection engages only when the operator scopes the query to a
    // Spirit (`--spirit <name>`). Bare `maosctl audit query` keeps the
    // legacy raw `AuditEntry` NDJSON surface (Story 1b.1 / `to_ndjson`) so
    // existing tooling (e.g. `tests/integration/audit_spine_smoke.sh`)
    // observing `frame_id`/`intent` continues to work. AC1 mandates the
    // FR4 six-key schema for the `--spirit` form specifically; the bare
    // form remains Story 9.1's territory.
    //
    // `_color` is currently advisory — both formats already emit zero ANSI
    // bytes unconditionally. Wired through for future colored ndjson keys
    // (Story 9.1) and to document the contract.
    let fr4_mode = spirit.is_some();
    let write_result = match (fr4_mode, format) {
        (true, AuditFormat::Ndjson) => maos_audit::to_fr4_ndjson(entries, lock),
        (true, AuditFormat::Plain) => maos_audit::to_fr4_plain(entries, lock),
        (false, AuditFormat::Ndjson) => maos_audit::to_ndjson(entries, lock),
        (false, AuditFormat::Plain) => maos_audit::to_plain(entries, lock),
    };
    match write_result {
        Ok(()) => ExitCode::SUCCESS,
        Err(maos_audit::AuditError::Fr4SchemaViolation {
            line,
            missing_field,
        }) => {
            eprintln!(
                "maosctl: audit query — FR4 schema violation at line {line}: missing field '{missing_field}'"
            );
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("maosctl: audit query — output error: {e}");
            ExitCode::from(2)
        }
    }
}

/// Resolve the default Transparency Log SQLite path.
///
/// Delegates to [`maos_audit::default_transparency_log_path`] — the single
/// source of truth shared by `maos-bin` (write side) and `maos-cli` (read
/// side) to prevent path-drift data loss.
fn default_transparency_log_path() -> PathBuf {
    maos_audit::default_transparency_log_path()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::ColorChoice;
    use crate::cli::{Cli, InstallArgs, RunArgs, Subcommand};
    use clap::Parser;

    #[test]
    fn dispatch_run_hello_spirit() {
        // Verify the dispatch maps 'run hello-spirit' to the run handler
        let cli = Cli::try_parse_from(["maosctl", "run", "hello-spirit"]).unwrap();
        match &cli.command {
            Subcommand::Run(args) => {
                assert_eq!(args.spirit.as_deref(), Some("hello-spirit"));
            }
            _ => panic!("expected Run subcommand"),
        }
    }

    #[test]
    fn dispatch_install() {
        let cli = Cli::try_parse_from(["maosctl", "install"]).unwrap();
        match &cli.command {
            Subcommand::Install(_args) => {}
            _ => panic!("expected Install subcommand"),
        }
    }

    #[test]
    fn dispatch_unknown_spirit_run() {
        // Verify the dispatch handles unknown spirit names gracefully
        let color = ColorChoice::Auto;
        let args = RunArgs {
            spirit: Some("nonexistent-spirit".into()),
            args: vec![],
        };
        let result = run(&args, color);
        // Non-zero exit code expected
        assert_ne!(result, ExitCode::SUCCESS);
    }

    #[test]
    fn dispatch_unknown_spirit_install() {
        let color = ColorChoice::Auto;
        let args = InstallArgs {
            source: Some("nonexistent-spirit".into()),
        };
        let result = install(&args, color);
        // Non-zero exit code expected
        assert_ne!(result, ExitCode::SUCCESS);
    }

    // ── FR4 audit query dispatch parsing tests (Story 1b.5b) ─────────

    #[test]
    fn audit_query_accepts_spirit_and_format_flags() {
        use crate::cli::{AuditFormat, AuditQuery};
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "ndjson",
        ])
        .expect("audit query --spirit / --format must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit, format }) => {
                    assert_eq!(spirit.as_deref(), Some("hello-spirit"));
                    assert_eq!(*format, AuditFormat::Ndjson);
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_accepts_plain_format() {
        use crate::cli::{AuditFormat, AuditQuery};
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "query",
            "--spirit",
            "hello-spirit",
            "--format",
            "plain",
        ])
        .expect("audit query --format plain must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit: _, format }) => {
                    assert_eq!(*format, AuditFormat::Plain);
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_defaults_format_to_ndjson() {
        use crate::cli::{AuditFormat, AuditQuery};
        let cli = Cli::try_parse_from(["maosctl", "audit", "query"])
            .expect("audit query with no flags must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { spirit, format }) => {
                    assert!(spirit.is_none(), "no --spirit means None");
                    assert_eq!(*format, AuditFormat::Ndjson, "default format is ndjson");
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn resolve_spirit_pid_maps_hello_spirit_to_zero() {
        assert_eq!(resolve_spirit_pid("hello-spirit").unwrap(), 0);
    }

    #[test]
    fn resolve_spirit_pid_rejects_other_names_with_clear_diagnostic() {
        let err = resolve_spirit_pid("orchestrator").unwrap_err();
        assert!(
            err.contains("only 'hello-spirit' is available at v0.1-β"),
            "diagnostic must name the v0.1-β scope: got {err}"
        );
    }

    // ── Lifecycle verb parsing tests (Story 1b.5c, AC1) ──────────────────

    #[test]
    fn dispatch_start_parses_hello_spirit_globally_with_plain() {
        let cli = Cli::try_parse_from(["maosctl", "--plain", "start", "hello-spirit"]).unwrap();
        assert!(cli.plain, "global --plain flag must round-trip");
        match &cli.command {
            Subcommand::Start(args) => {
                assert_eq!(args.spirit.as_deref(), Some("hello-spirit"));
            }
            _ => panic!("expected Start subcommand"),
        }
    }

    #[test]
    fn dispatch_stop_unload_parse_with_no_args() {
        // The CLI parses even without a spirit name; the dispatch helper
        // surfaces the missing-arg diagnostic at runtime (verified via
        // integration smoke). This unit test pins the clap surface.
        let cli = Cli::try_parse_from(["maosctl", "stop"]).unwrap();
        match &cli.command {
            Subcommand::Stop(args) => assert!(args.spirit.is_none()),
            _ => panic!("expected Stop subcommand"),
        }
        let cli = Cli::try_parse_from(["maosctl", "unload"]).unwrap();
        match &cli.command {
            Subcommand::Unload(args) => assert!(args.spirit.is_none()),
            _ => panic!("expected Unload subcommand"),
        }
    }

    #[test]
    fn lifecycle_verb_rejects_unknown_spirit_with_exit_two() {
        // Drive the helper directly — `resolve_spirit_pid` rejects unknown
        // names with the exact v0.1-β diagnostic, and the helper translates
        // that to exit 2 BEFORE spawning the child.
        let color = ColorChoice::Auto;
        let code = lifecycle_verb("start", Some("orchestrator"), color);
        assert_ne!(code, ExitCode::SUCCESS);
    }

    #[test]
    fn lifecycle_verb_rejects_missing_spirit_with_exit_two() {
        let color = ColorChoice::Auto;
        let code = lifecycle_verb("stop", None, color);
        assert_ne!(code, ExitCode::SUCCESS);
    }
}
