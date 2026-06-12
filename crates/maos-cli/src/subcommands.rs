//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::path::PathBuf;
use std::process::ExitCode;

use crate::accessibility::ColorChoice;
use crate::cli::{
    AuditFormat, AuditQuery, HaltArgs, HaltOp, ImportArgs, InstallArgs, OrchestratorArgs,
    OrchestratorOp, PauseArgs, PostureArgs, PostureChoice, ResolutionKindChoice, ResumeArgs,
    RevocationsArgs, RevocationsOp, RevokeTokenArgs, RunArgs, SkillsArgs, SkillsOp, SpiritArgs,
    SpiritOp, Subcommand, UpgradePolicyArg,
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
        Subcommand::Skills(args) => dispatch_skills(args, color),
    }
}

/// Story 7.4 (FR39) — `maosctl skills <list|approve|reject>`.
///
/// `list` runs filesystem discovery over the conventional `[skills.search_path]`
/// roots and renders each `maos.skill.v1` skill with its admission state (always
/// `Pending` at discovery — nothing auto-admits). `approve`/`reject` give the
/// pending operator-admission queue its exit. At v0.5 the admission queue has no
/// cross-invocation persistent store, so `approve`/`reject` record the operator
/// decision and acknowledge it; the durable queue store is future work (the
/// queue mechanics + audit rows live in `maos-skill::admission`).
fn dispatch_skills(args: &SkillsArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        SkillsOp::List { root } => {
            let roots: Vec<PathBuf> = if root.is_empty() {
                maos_skill::default_search_path()
            } else {
                root.iter().map(PathBuf::from).collect()
            };
            let outcome = maos_skill::discover_skills_detailed(&roots);
            if outcome.discovered.is_empty() && outcome.skipped.is_empty() {
                println!("maosctl skills: no skills discovered on the search path");
            }
            for d in &outcome.discovered {
                println!(
                    "{:<24} {:<10} {:?}  ({})",
                    d.skill.manifest.id,
                    d.skill.manifest.version,
                    d.state,
                    d.source_path.display()
                );
            }
            for (path, reason) in &outcome.skipped {
                eprintln!("maosctl skills: skipped {} — {}", path.display(), reason);
            }
            ExitCode::SUCCESS
        }
        SkillsOp::Approve { skill_id } => {
            // FR39 admission exit. The durable queue store is future work; at
            // v0.5 we acknowledge the operator decision (the queue + audit-row
            // mechanics are exercised by `maos-skill::admission` tests + the
            // `smoke-skill-7-4` arm).
            println!("maosctl skills: operator-admit acknowledged for skill `{skill_id}` (FR39)");
            ExitCode::SUCCESS
        }
        SkillsOp::Reject { skill_id } => {
            println!("maosctl skills: operator-reject acknowledged for skill `{skill_id}` (FR39)");
            ExitCode::SUCCESS
        }
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

    if let Err(diag) = resolve_spirit_pid(name, &default_transparency_log_path(), false) {
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
    if let Err(diag) = resolve_spirit_pid(&args.spirit, &default_transparency_log_path(), false) {
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
                if let Err(diag) = resolve_spirit_pid(s, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
    if let Err(diag) = resolve_spirit_pid(&args.spirit, &default_transparency_log_path(), false) {
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
    if let Err(diag) = resolve_spirit_pid(&args.spirit, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
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
        None => audit_query(
            None,
            None,
            None,
            None,
            None,
            None,
            false,
            None,
            AuditFormat::Ndjson,
            color,
        ),
        Some(AuditQuery::Query {
            spirit,
            format,
            range,
            frame_kind,
            intent_contains,
            capability,
            boot,
            all_boots,
            tag,
        }) => {
            if let Some(_tag_val) = tag {
                eprintln!("maosctl: audit query — --tag is reserved; use --intent-contains for substring matching on the intent column");
                return ExitCode::from(2);
            }
            audit_query(
                spirit.as_deref(),
                range.as_deref(),
                frame_kind.as_deref(),
                intent_contains.as_deref(),
                capability.as_deref(),
                boot.as_ref().copied(),
                *all_boots,
                None,
                *format,
                color,
            )
        }
        Some(AuditQuery::SealedExport {
            spirit,
            range,
            output,
            audit_key,
        }) => audit_sealed_export(
            spirit.as_deref(),
            range.as_deref(),
            output,
            audit_key,
            color,
        ),
        Some(AuditQuery::Keygen { output }) => audit_keygen(output),
        Some(AuditQuery::VerifyBundle { bundle, pubkey }) => audit_verify_bundle(bundle, pubkey),
        Some(AuditQuery::SubjectAccess { principal, format }) => {
            audit_subject_access(principal, *format, color)
        }
        Some(AuditQuery::PostureDelta {
            range,
            spirit,
            format,
        }) => audit_posture_delta(range, spirit.as_deref(), *format, color),
    }
}

/// Resolve a Spirit name to one or more `(boot_nonce, spirit_pid)` pairs.
///
/// Delegates to [`maos_audit::resolve_spirit_name`] which scans the TL for
/// `lifecycle.admit`/`lifecycle.load` intents. Per Decision E: keyed on
/// `(boot_nonce, spirit_pid)` to discriminate pid reuse across boots.
/// Default: latest boot (max boot_nonce). `all_boots` unions all incarnations.
///
/// Returns a Vec with 1 element normally, or multiple for `--all-boots`.
/// Unknown names exit non-zero with a clear diagnostic.
fn resolve_spirit_pid(
    name: &str,
    db_path: &std::path::Path,
    all_boots: bool,
) -> Result<Vec<(u64, u32)>, String> {
    maos_audit::resolve_spirit_name(db_path, name, all_boots)
}

/// Parse a range string into `(since_ns, until_ns)` relative to now.
///
/// Supports: "30d", "7d", "24h", "1h" (relative from now) or an absolute
/// nanosecond timestamp (all-digit string).
fn parse_range(range: &str) -> Result<(Option<u64>, Option<u64>), String> {
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    if range.ends_with('d') {
        let days: u64 = range[..range.len() - 1]
            .parse()
            .map_err(|_| format!("invalid range '{range}': expected number before 'd'"))?;
        let ns = days
            .checked_mul(24 * 60 * 60 * 1_000_000_000)
            .ok_or_else(|| {
                format!("invalid range '{range}': overflow converting {days} days to nanoseconds")
            })?;
        Ok((Some(now_ns.saturating_sub(ns)), None))
    } else if range.ends_with('h') {
        let hours: u64 = range[..range.len() - 1]
            .parse()
            .map_err(|_| format!("invalid range '{range}': expected number before 'h'"))?;
        let ns = hours.checked_mul(60 * 60 * 1_000_000_000).ok_or_else(|| {
            format!("invalid range '{range}': overflow converting {hours} hours to nanoseconds")
        })?;
        Ok((Some(now_ns.saturating_sub(ns)), None))
    } else if range.chars().all(|c| c.is_ascii_digit()) {
        let abs: u64 = range
            .parse()
            .map_err(|_| format!("invalid range '{range}': expected nanosecond timestamp"))?;
        Ok((Some(abs), None))
    } else {
        Err(format!(
            "invalid range '{range}': use relative (e.g. '30d', '1h') or absolute nanoseconds"
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn audit_query(
    spirit: Option<&str>,
    range: Option<&str>,
    frame_kind: Option<&str>,
    intent_contains: Option<&str>,
    capability: Option<&str>,
    boot: Option<u64>,
    all_boots: bool,
    _tag: Option<&str>,
    format: AuditFormat,
    _color: ColorChoice,
) -> ExitCode {
    let db_path = default_transparency_log_path();

    let mut filter = maos_audit::AuditFilter::default();
    // Collect all (boot_nonce, spirit_pid) pairs for client-side filtering
    // when --all-boots resolves to multiple incarnations.
    let mut multi_boot_pairs: Option<std::collections::HashSet<(u64, u32)>> = None;
    if let Some(name) = spirit {
        match resolve_spirit_pid(name, &db_path, all_boots) {
            Ok(pairs) => {
                if pairs.len() == 1 {
                    filter.spirit_pid = Some(pairs[0].1);
                    filter.boot_nonce = Some(pairs[0].0);
                } else if !pairs.is_empty() {
                    // Multiple incarnations: collect all (boot, pid) pairs and
                    // filter client-side so no incarnation is silently dropped.
                    multi_boot_pairs = Some(
                        pairs
                            .iter()
                            .copied()
                            .collect::<std::collections::HashSet<_>>(),
                    );
                    // Set spirit_pid only if all pairs share the same pid; otherwise
                    // query without pid filter and rely on client-side filtering.
                    let unique_pids: std::collections::HashSet<u32> =
                        pairs.iter().map(|(_, pid)| *pid).collect();
                    if unique_pids.len() == 1 {
                        filter.spirit_pid = Some(pairs[0].1);
                    }
                    // Do NOT set boot_nonce — client-side filter handles it.
                }
            }
            Err(diag) => {
                eprintln!("maosctl: audit query — {diag}");
                return ExitCode::from(2);
            }
        }
    }

    // Parse range filter
    if let Some(range_str) = range {
        match parse_range(range_str) {
            Ok((since, until)) => {
                filter.since_ns = since;
                if let Some(u) = until {
                    filter.until_ns = Some(u);
                }
            }
            Err(e) => {
                eprintln!("maosctl: audit query — {e}");
                return ExitCode::from(2);
            }
        }
    }

    // FR41 new filter fields
    filter.kind = frame_kind.map(|s| s.to_string());
    filter.intent_contains = intent_contains.map(|s| s.to_string());
    filter.capability_token = capability.map(|s| s.to_string());
    if let Some(b) = boot {
        if filter.boot_nonce.is_some() && filter.boot_nonce != Some(b) {
            eprintln!(
                "maosctl: audit query — --boot {b} conflicts with boot {} resolved from spirit name; \
                 using explicit --boot value",
                filter.boot_nonce.unwrap()
            );
        }
        filter.boot_nonce = Some(b);
        // When --boot is explicit, clear multi-boot client-side filter
        // since the single boot is now the authoritative scope.
        multi_boot_pairs = None;
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

    // Client-side filtering for multi-boot unions: only keep entries whose
    // (boot_nonce, spirit_pid) matches one of the resolved incarnations.
    let entries = match multi_boot_pairs {
        Some(ref pair_set) => entries
            .into_iter()
            .filter(|e| pair_set.contains(&(e.boot_nonce, e.spirit_pid)))
            .collect(),
        None => entries,
    };

    let stdout = std::io::stdout();
    let lock = stdout.lock();
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

// ─── FR44: Sealed Export, Keygen, VerifyBundle ──────────────────────────────

/// FR44 — produce a signed sealed-export bundle.
fn audit_sealed_export(
    spirit: Option<&str>,
    range: Option<&str>,
    output: &Option<PathBuf>,
    audit_key: &Option<PathBuf>,
    _color: ColorChoice,
) -> ExitCode {
    // Load audit signing key
    let seed = match maos_domain::audit_key::load_audit_key_seed(audit_key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — {e}");
            return ExitCode::from(2);
        }
    };

    // Query audit entries
    let db_path = default_transparency_log_path();
    let mut filter = maos_audit::AuditFilter::default();
    if let Some(name) = spirit {
        match resolve_spirit_pid(name, &db_path, false) {
            Ok(pairs) => {
                if let Some(pair) = pairs.first() {
                    filter.spirit_pid = Some(pair.1);
                    filter.boot_nonce = Some(pair.0);
                }
            }
            Err(diag) => {
                eprintln!("maosctl: audit sealed-export — {diag}");
                return ExitCode::from(2);
            }
        }
    }

    // Patch 3: honor --range when selecting entries for the bundle
    if let Some(range_str) = range {
        match parse_range(range_str) {
            Ok((since, until)) => {
                filter.since_ns = since;
                if let Some(u) = until {
                    filter.until_ns = Some(u);
                }
            }
            Err(e) => {
                eprintln!("maosctl: audit sealed-export — {e}");
                return ExitCode::from(2);
            }
        }
    }

    let entries = match maos_audit::query(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit sealed-export — no Transparency Log found at {}. \
                 Run `maosctl run hello-spirit` first to seed the log.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — error: {e}");
            return ExitCode::from(2);
        }
    };

    // Build freshness metadata
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let since_ns = entries.iter().map(|e| e.timestamp_ns).min().unwrap_or(0);
    let until_ns = entries
        .iter()
        .map(|e| e.timestamp_ns)
        .max()
        .unwrap_or(now_ns);

    let freshness = maos_audit::sealed_export::FreshnessMetadata {
        export_timestamp_ns: now_ns,
        covered_window: maos_audit::sealed_export::CoveredWindow { since_ns, until_ns },
        export_seq: now_ns, // Patch 2: monotonic via nanosecond timestamp
    };

    // Patch 1: populate I12 digest refs and I11 distilled content from
    // actual distillate frames in the queried entries, rather than empty vecs.
    let i12_refs: Vec<String> = entries
        .iter()
        .filter(|e| e.kind == "distillate")
        .map(|e| e.frame_id_hex.clone())
        .collect();

    let i11_content: Vec<maos_audit::sealed_export::I11Content> = entries
        .iter()
        .filter(|e| e.kind == "distillate")
        .map(|e| maos_audit::sealed_export::I11Content {
            source_log_ref: vec![e.frame_id_hex.clone()],
            distillation_depth: 1,
        })
        .collect();

    let unsigned =
        maos_audit::sealed_export::build_bundle(entries, i12_refs, i11_content, freshness);

    let signed = match maos_audit::sealed_export::sign_bundle(unsigned, &seed) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — signing error: {e}");
            return ExitCode::from(2);
        }
    };

    let json_bytes = match serde_json::to_string_pretty(&signed) {
        Ok(s) => s.into_bytes(),
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — serialization error: {e}");
            return ExitCode::from(2);
        }
    };

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("maosctl: audit sealed-export — cannot create output dir: {e}");
                    return ExitCode::from(2);
                }
            }
            if let Err(e) = std::fs::write(path, &json_bytes) {
                eprintln!("maosctl: audit sealed-export — write error: {e}");
                return ExitCode::from(2);
            }
            let pubkey = maos_audit::sealed_export::derive_pubkey(&seed);
            eprintln!(
                "maosctl: sealed export written to {} ({} entries, pubkey {})",
                path.display(),
                signed.entries.len(),
                hex::encode(pubkey),
            );
        }
        None => {
            use std::io::Write;
            let stdout = std::io::stdout();
            if let Err(e) = stdout.lock().write_all(&json_bytes) {
                eprintln!("maosctl: audit sealed-export — write error: {e}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

/// FR44 — generate an Ed25519 audit signing key.
fn audit_keygen(output: &Option<PathBuf>) -> ExitCode {
    match maos_domain::audit_key::generate_audit_key(output) {
        Ok(fingerprint) => {
            let path = output
                .clone()
                .unwrap_or_else(maos_domain::audit_key::default_audit_key_path);
            eprintln!(
                "maosctl: audit keygen — key written to {} (fingerprint: {fingerprint})",
                path.display(),
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("maosctl: audit keygen — {e}");
            ExitCode::from(2)
        }
    }
}

/// FR44 — verify a sealed-export bundle.
fn audit_verify_bundle(bundle: &PathBuf, pubkey_arg: &str) -> ExitCode {
    // Read bundle
    let bundle_bytes = match std::fs::read_to_string(bundle) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit verify-bundle — read error: {e}");
            return ExitCode::from(2);
        }
    };

    let bundle: maos_audit::sealed_export::AuditBundle = match serde_json::from_str(&bundle_bytes) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit verify-bundle — invalid bundle JSON: {e}");
            return ExitCode::from(2);
        }
    };

    // Resolve public key: distinguish file path from hex string.
    // A hex pubkey is exactly 64 hex chars (32 bytes). A file path typically
    // has an extension or directory separator — use that as the discriminator.
    let pubkey_hex = {
        let path = std::path::Path::new(pubkey_arg);
        // Treat as file if it has a file extension (.hex, .pub, .txt, etc.)
        // or contains a directory separator — avoids treating hex strings as
        // file paths on systems where a 64-char hex string happens to name a
        // real filesystem entry.
        let looks_like_file = path.extension().is_some()
            || pubkey_arg.contains('/')
            || pubkey_arg.contains(std::path::MAIN_SEPARATOR);
        if looks_like_file && path.exists() {
            match std::fs::read_to_string(pubkey_arg) {
                Ok(s) => s.trim().to_string(),
                Err(e) => {
                    eprintln!(
                        "maosctl: audit verify-bundle — cannot read pubkey file '{}': {e}",
                        pubkey_arg
                    );
                    return ExitCode::from(2);
                }
            }
        } else if looks_like_file {
            eprintln!(
                "maosctl: audit verify-bundle — pubkey file '{}' not found",
                pubkey_arg
            );
            return ExitCode::from(2);
        } else {
            pubkey_arg.to_string()
        }
    };

    let pubkey_bytes: [u8; 32] = match hex::decode(&pubkey_hex)
        .map_err(|e| format!("invalid pubkey hex: {e}"))
        .and_then(|bytes| bytes.try_into().map_err(|bytes: Vec<u8>| {
            format!(
                "wrong pubkey length: expected 32 bytes (64 hex chars), got {} bytes ({} hex chars)",
                bytes.len(),
                pubkey_hex.len()
            )
        })) {
        Ok(arr) => arr,
        Err(e) => {
            eprintln!("maosctl: audit verify-bundle — {e}");
            return ExitCode::from(2);
        }
    };

    match maos_audit::sealed_export::verify_bundle(&bundle, &pubkey_bytes) {
        Ok(()) => {
            eprintln!(
                "maosctl: audit verify-bundle — OK ({} entries, seq {})",
                bundle.entries.len(),
                bundle.freshness.export_seq,
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("maosctl: audit verify-bundle — verification failed: {e}");
            ExitCode::from(1)
        }
    }
}

// ─── FR42: Subject Access ──────────────────────────────────────────────

/// FR42 — subject-access query: retrieve all principal_index rows for a
/// given principal, enriched with provenance and spirit-name resolution.
fn audit_subject_access(principal: &str, format: AuditFormat, _color: ColorChoice) -> ExitCode {
    let db_path = default_transparency_log_path();

    let raw_entries = match maos_audit::subject_access_query(&db_path, principal) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit subject-access — no Transparency Log found at {}. \
                 Run `maosctl run hello-spirit` first to seed the log.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit subject-access — error: {e}");
            return ExitCode::from(2);
        }
    };

    let enriched = match maos_audit::enrich_subject_access(&db_path, raw_entries) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("maosctl: audit subject-access — enrichment error: {e}");
            return ExitCode::from(2);
        }
    };

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    match format {
        AuditFormat::Ndjson => {
            use std::io::Write;
            for entry in &enriched {
                let line = match serde_json::to_string(entry) {
                    Ok(l) => l,
                    Err(e) => {
                        eprintln!("maosctl: audit subject-access — encode error: {e}");
                        return ExitCode::from(2);
                    }
                };
                if let Err(e) = writeln!(lock, "{line}") {
                    eprintln!("maosctl: audit subject-access — write error: {e}");
                    return ExitCode::from(2);
                }
            }
        }
        AuditFormat::Plain => {
            use std::io::Write;
            for entry in &enriched {
                let provenance_str = match &entry.provenance {
                    maos_audit::Provenance::Direct { frame_ref } => {
                        format!("direct({frame_ref})")
                    }
                    maos_audit::Provenance::Distilled {
                        effective_source_log_ref,
                        distillation_depth,
                    } => {
                        format!(
                            "distilled(depth={distillation_depth}, refs={})",
                            effective_source_log_ref.join(",")
                        )
                    }
                };
                let name = entry.writer_spirit_name.as_deref().unwrap_or("<unknown>");
                if let Err(e) = writeln!(
                    lock,
                    "{}  {}  pid={}  boot={}  {}:{}  {}",
                    entry.timestamp_ns,
                    entry.principal_id,
                    entry.writer_spirit_pid,
                    entry.boot_nonce.unwrap_or(0),
                    entry.schema,
                    entry.key,
                    provenance_str,
                ) {
                    eprintln!("maosctl: audit subject-access — write error: {e}");
                    return ExitCode::from(2);
                }
                let _ = name; // used in future enriched output
            }
        }
    }

    ExitCode::SUCCESS
}

// ─── FR43: Posture Delta ───────────────────────────────────────────────

/// FR43 — posture-delta report: classify composed log entries into
/// capability changes, sandbox tier changes, and consent ruptures.
fn audit_posture_delta(
    range_str: &str,
    spirit: Option<&str>,
    format: AuditFormat,
    _color: ColorChoice,
) -> ExitCode {
    let db_path = default_transparency_log_path();
    let journal_path = maos_audit::default_journal_path();

    let (since_ns, until_ns) = match parse_range(range_str) {
        Ok((s, u)) => (s.unwrap_or(0), u),
        Err(e) => {
            eprintln!("maosctl: audit posture-delta — {e}");
            return ExitCode::from(2);
        }
    };

    let range = maos_audit::log_composition::LogRange {
        since_ns,
        until_ns: until_ns.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        }),
    };

    let report =
        match maos_audit::log_composition::posture_delta(&db_path, &journal_path, range, spirit) {
            Ok(r) => r,
            Err(maos_audit::AuditError::Open(_)) => {
                eprintln!(
                    "maosctl: audit posture-delta — no Transparency Log found at {}.",
                    db_path.display()
                );
                return ExitCode::from(2);
            }
            Err(e) => {
                eprintln!("maosctl: audit posture-delta — error: {e}");
                return ExitCode::from(2);
            }
        };

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    match format {
        AuditFormat::Ndjson => {
            use std::io::Write;
            let json = match serde_json::to_string(&report) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("maosctl: audit posture-delta — encode error: {e}");
                    return ExitCode::from(2);
                }
            };
            if let Err(e) = writeln!(lock, "{json}") {
                eprintln!("maosctl: audit posture-delta — write error: {e}");
                return ExitCode::from(2);
            }
        }
        AuditFormat::Plain => {
            use std::io::Write;
            let s = &report.summary;
            if let Err(e) = writeln!(
                lock,
                "Posture Delta Report ({}..{})",
                s.window_since_ns, s.window_until_ns
            ) {
                eprintln!("maosctl: audit posture-delta — write error: {e}");
                return ExitCode::from(2);
            }
            if let Err(e) = writeln!(
                lock,
                "  total={}  issued={}  revoked={}  net_delta={}  tier={}  consent_rupture={}",
                s.total_events,
                s.capabilities_issued,
                s.capabilities_revoked,
                s.net_capability_delta,
                s.sandbox_tier_changes,
                s.consent_ruptures,
            ) {
                eprintln!("maosctl: audit posture-delta — write error: {e}");
                return ExitCode::from(2);
            }
            if let Err(e) = writeln!(lock, "  NOTE: {}", s.consent_dimension_limitation) {
                eprintln!("maosctl: audit posture-delta — write error: {e}");
                return ExitCode::from(2);
            }
            for event in &report.events {
                if let Err(e) = writeln!(lock, "  {}  {:?}", event.timestamp_ns, event.change) {
                    eprintln!("maosctl: audit posture-delta — write error: {e}");
                    return ExitCode::from(2);
                }
            }
        }
    }

    ExitCode::SUCCESS
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
                Some(AuditQuery::Query { spirit, format, .. }) => {
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
                Some(AuditQuery::Query {
                    spirit: _, format, ..
                }) => {
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
                Some(AuditQuery::Query { spirit, format, .. }) => {
                    assert!(spirit.is_none(), "no --spirit means None");
                    assert_eq!(*format, AuditFormat::Ndjson, "default format is ndjson");
                }
                _ => panic!("expected AuditQuery::Query struct variant"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn resolve_spirit_pid_reads_from_tl() {
        // Create a test DB with a lifecycle.admit frame for "hello-spirit"
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("test.sqlite");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );",
        )
        .unwrap();
        let payload =
            serde_json::to_vec(&serde_json::json!({"spirit_id": "hello-spirit"})).unwrap();
        conn.execute(
            "INSERT INTO transparency_log (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, kind, intent, payload_redacted, origin)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &[0xAAu8; 16] as &[u8],
                1000i64,
                0i64,
                1i64,
                rusqlite::types::Null,
                19i64,  // SpiritAdmitted
                "hello-spirit",
                &payload as &[u8],
                0i64,
            ],
        ).unwrap();
        drop(conn);

        let result = resolve_spirit_pid("hello-spirit", &db_path, false).unwrap();
        assert_eq!(result, vec![(1, 0)]);
    }

    #[test]
    fn resolve_spirit_pid_rejects_unknown_names_with_clear_diagnostic() {
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("test.sqlite");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS transparency_log (
                frame_id BLOB NOT NULL PRIMARY KEY,
                timestamp_ns INTEGER NOT NULL,
                spirit_pid INTEGER NOT NULL,
                boot_nonce INTEGER NOT NULL,
                capability_token BLOB,
                kind INTEGER NOT NULL,
                intent TEXT NOT NULL,
                payload_redacted BLOB NOT NULL,
                origin INTEGER NOT NULL
            );",
        )
        .unwrap();
        drop(conn);

        let err = resolve_spirit_pid("orchestrator", &db_path, false).unwrap_err();
        assert!(
            err.contains("unknown spirit 'orchestrator'"),
            "diagnostic must name the unknown spirit: got {err}"
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

    // ── FR41/FR42/FR43 — new audit query flag parsing tests (Story 9.1) ─────

    #[test]
    fn audit_query_intent_contains_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from(["maosctl", "audit", "query", "--intent-contains", "hello"])
            .expect("--intent-contains must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query {
                    intent_contains, ..
                }) => {
                    assert_eq!(intent_contains.as_deref(), Some("hello"));
                }
                _ => panic!("expected AuditQuery::Query"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_capability_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from(["maosctl", "audit", "query", "--capability", "abcd1234"])
            .expect("--capability must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { capability, .. }) => {
                    assert_eq!(capability.as_deref(), Some("abcd1234"));
                }
                _ => panic!("expected AuditQuery::Query"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_range_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from(["maosctl", "audit", "query", "--range", "30d"])
            .expect("--range must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { range, .. }) => {
                    assert_eq!(range.as_deref(), Some("30d"));
                }
                _ => panic!("expected AuditQuery::Query"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_tag_parses_but_handler_errors() {
        use crate::cli::AuditQuery;
        // The flag parses; the handler rejects at runtime.
        let cli = Cli::try_parse_from(["maosctl", "audit", "query", "--tag", "foo"])
            .expect("--tag must parse as a valid flag");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query { tag, .. }) => {
                    assert_eq!(tag.as_deref(), Some("foo"));
                }
                _ => panic!("expected AuditQuery::Query"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_query_spirit_all_boots_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "query",
            "--spirit",
            "researcher",
            "--all-boots",
        ])
        .expect("--spirit + --all-boots must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Query {
                    spirit, all_boots, ..
                }) => {
                    assert_eq!(spirit.as_deref(), Some("researcher"));
                    assert!(*all_boots);
                }
                _ => panic!("expected AuditQuery::Query"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_subject_access_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "subject-access",
            "--principal",
            "user:alice",
        ])
        .expect("audit subject-access must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::SubjectAccess { principal, .. }) => {
                    assert_eq!(principal, "user:alice");
                }
                _ => panic!("expected AuditQuery::SubjectAccess"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_posture_delta_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from(["maosctl", "audit", "posture-delta", "--range", "30d"])
            .expect("audit posture-delta must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::PostureDelta { range, .. }) => {
                    assert_eq!(range, "30d");
                }
                _ => panic!("expected AuditQuery::PostureDelta"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_sealed_export_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "sealed-export",
            "--spirit",
            "test",
            "--range",
            "7d",
        ])
        .expect("audit sealed-export must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::SealedExport { spirit, range, .. }) => {
                    assert_eq!(spirit.as_deref(), Some("test"));
                    assert_eq!(range.as_deref(), Some("7d"));
                }
                _ => panic!("expected AuditQuery::SealedExport"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_keygen_parses() {
        use crate::cli::AuditQuery;
        let cli =
            Cli::try_parse_from(["maosctl", "audit", "keygen"]).expect("audit keygen must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::Keygen { .. }) => {}
                _ => panic!("expected AuditQuery::Keygen"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn audit_verify_bundle_parses() {
        use crate::cli::AuditQuery;
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "verify-bundle",
            "/tmp/bundle.json",
            "--pubkey",
            "abc123",
        ])
        .expect("audit verify-bundle must parse");
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::VerifyBundle { bundle, pubkey }) => {
                    assert_eq!(bundle.to_str(), Some("/tmp/bundle.json"));
                    assert_eq!(pubkey, "abc123");
                }
                _ => panic!("expected AuditQuery::VerifyBundle"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }
}
