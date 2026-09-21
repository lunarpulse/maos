//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

// Story 9.7 — trait must be in scope for `load()`/`save()` on the store.
use maos_domain::invariants::i4::ApprovalDecision;
use maos_iac::adapter::transparency_log::TransparencyLogAdapter;
use maos_skill::SkillQueueStore;
use maos_skill::{DiscoveredSkill, SkillAdmissionState, SkillId};
use std::collections::{HashMap, HashSet};

use crate::accessibility::ColorChoice;
use crate::cli::{
    AuditFormat, AuditQuery, BackupArgs, BackupOp, ForgetArgs, GovernanceArgs, GovernanceOp,
    HaltArgs, HaltOp, ImportArgs, InstallArgs, LegalHoldArgs, LegalHoldOp, LoadArgs, MigrateArgs,
    MigrateOp, OrchestratorArgs, OrchestratorOp, PauseArgs, PostureArgs, PostureChoice,
    ResolutionKindChoice, ResumeArgs, RevocationsArgs, RevocationsOp, RevokeTokenArgs, RunArgs,
    SkillsArgs, SkillsOp, SpiritArgs, SpiritOp, Subcommand, UninstallArgs, UpgradePolicyArg,
};
use crate::door_client::{self, DoorConfig, DoorError, DEFAULT_ROUTE_BUDGET, LONG_ROUTE_BUDGET};

const MAX_DOOR_BODY_BYTES: usize = 64 * 1024;

pub fn dispatch(cmd: &Subcommand, color: ColorChoice) -> ExitCode {
    match cmd {
        Subcommand::Install(args) => install(args, color),
        Subcommand::Load(args) => dispatch_load(args, color),
        Subcommand::Start(args) => lifecycle_door_verb("start", args.spirit.as_deref(), color),
        Subcommand::Stop(_) => refuse_stop(),
        Subcommand::Unload(args) => lifecycle_door_verb("unload", args.spirit.as_deref(), color),
        Subcommand::Uninstall(args) => dispatch_uninstall(args, color),
        Subcommand::Forget(args) => dispatch_forget(args, color),
        Subcommand::LegalHold(args) => dispatch_legal_hold(args, color),
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
        Subcommand::Governance(args) => dispatch_governance(args, color),
        Subcommand::Backup(args) => dispatch_backup(args, color),
        Subcommand::Migrate(args) => dispatch_migrate(args, color),
        Subcommand::ReleasePubkey => release_pubkey(),
        Subcommand::Cohort(args) => dispatch_cohort(args, color),
    }
}

// ─── Story 16-1 — the door (D-16-1-A): shared client plumbing ───────────

/// D-16-1-I — `stop` is refused client-side BEFORE the accessibility smoke
/// short-circuit (which would otherwise keep printing "stop smoke ok"):
/// exit 2, no journal row, no round trip. `ScbLifecycleState` has no stopped
/// state and FR9 lists load/start/pause/resume/unload — a "stop" could only
/// mean a resumable pause or a silently destructive unload.
fn refuse_stop() -> ExitCode {
    eprintln!(
        "maosctl: no kernel transition is named stop — use `maosctl pause` (resumable) or `maosctl unload` (terminal)"
    );
    ExitCode::from(2)
}

/// Accessibility cascade short-circuit (Story 1b.5c): deterministic
/// ANSI-free output without touching the door, the journal or the TL.
fn smoke_short_circuit(verb: &str, name: &str) -> Option<ExitCode> {
    if std::env::var_os("MAOS_ACCESSIBILITY_SMOKE").is_some() {
        eprintln!("maosctl: {verb} smoke ok for {name}");
        return Some(ExitCode::SUCCESS);
    }
    None
}

/// Local twin of the door's route-segment validation: a request the daemon
/// must refuse never leaves maosctl. Exit 2 (local validation); the daemon
/// still re-validates because maosctl is not the only door client.
fn checked_id(verb: &str, id: &str) -> Option<ExitCode> {
    door_client::validate_id(id).err().map(|reason| {
        eprintln!("maosctl: {verb} — {reason}");
        ExitCode::from(2)
    })
}

/// D-16-1-W/X — the door receives a CANONICAL absolute manifest path: the
/// daemon resolves it in ITS working directory, not the operator's.
fn canonical_manifest(verb: &str, to: &str) -> Result<PathBuf, ExitCode> {
    let path = std::path::Path::new(to);
    if !path.exists() {
        eprintln!("maosctl: {verb} — manifest file not found: {to}");
        return Err(ExitCode::from(1));
    }
    std::fs::canonicalize(path).map_err(|error| {
        eprintln!("maosctl: {verb} — cannot canonicalize manifest {to}: {error}");
        ExitCode::from(1)
    })
}
/// Story 16-6 — `maosctl load <manifest>` (FR9's `load`).
///
/// `POST /v1/spirits` with `{"manifest": "<canonical absolute path>"}`: a
/// COLLECTION route, because `/v1/spirits/{id}/{verb}` cannot carry a load —
/// a filesystem path is not a legal path segment, and the id does not exist
/// until the daemon has parsed the manifest.
///
/// `LONG_ROUTE_BUDGET` because the daemon runs the whole `load → admit`
/// sequence inside the request, dispatching the Spirit's `on_load` hook with
/// its own manifest `[budget]` cap.
///
/// There is no `checked_id` here: the operator names a FILE, not an id.
fn dispatch_load(args: &LoadArgs, _color: ColorChoice) -> ExitCode {
    let manifest = match canonical_manifest("load", &args.manifest) {
        Ok(path) => path,
        Err(code) => return code,
    };
    let body = serde_json::to_vec(&serde_json::json!({
        "manifest": manifest.to_string_lossy(),
    }))
    .expect("serializing a hand-built Value cannot fail");
    door_verb("load", |config| {
        door_client::exchange(
            config,
            "POST",
            "/v1/spirits",
            Some(("application/json", &body)),
            LONG_ROUTE_BUDGET,
        )
    })
}

fn canonical_operator_file(verb: &str, label: &str, path: &str) -> Result<PathBuf, ExitCode> {
    std::fs::canonicalize(path).map_err(|error| {
        eprintln!("maosctl: {verb} — cannot canonicalize {label} {path}: {error}");
        ExitCode::from(1)
    })
}

/// Door-class verb (D-16-1-A): resolve the door, send, map. On
/// connect-refused run the momentary home-lock probe — `DaemonNotRunning`
/// vs `StoreInUse`, both 69 — because door-class verbs have NO offline arm:
/// their handlers are in-memory scheduler state no child can touch.
fn door_verb(
    verb: &str,
    request: impl FnOnce(&DoorConfig) -> Result<door_client::Response, DoorError>,
) -> ExitCode {
    match door_client::resolve_door() {
        Ok(config) => match request(&config) {
            Ok(response) => door_client::map_response(verb, response),
            Err(DoorError::ConnectRefused) => {
                door_client::door_class_connect_refused(verb, config.endpoint)
            }
            Err(error) => door_client::map_error(verb, error),
        },
        Err(error) => door_client::map_error(verb, error),
    }
}

/// Durable verb (AC5 / D-16-1-D / V-18): the door when it answers, the
/// offline child ONLY when nothing is configured (erasure never requires
/// `maos init`) or the connect is refused (the child takes the exclusive
/// store-lock set, which settles liveness without a race). A connected-but-
/// silent door (69) and a rejected bearer (77) are terminal — never fall
/// through to offline, because those are exactly the states in which a
/// daemon may still hold live memory.
fn durable_verb(
    verb: &str,
    request: impl FnOnce(&DoorConfig) -> Result<door_client::Response, DoorError>,
    offline: impl FnOnce() -> ExitCode,
) -> ExitCode {
    match door_client::resolve_door() {
        Ok(config) => match request(&config) {
            Ok(response) => door_client::map_response(verb, response),
            Err(DoorError::ConnectRefused) => offline(),
            Err(error) => door_client::map_error(verb, error),
        },
        Err(DoorError::NotInitialized(_)) => offline(),
        Err(error) => door_client::map_error(verb, error),
    }
}

/// `start`/`unload` over the door (D-16-1-A): the daemon resolves the name
/// (an unknown one is its typed 404) and performs the scheduler transition.
/// The accessibility smoke short-circuit stays; the TL preflight and the
/// hello-spirit guard are gone — they refused the very Spirits the door can
/// now name, before any round trip.
fn lifecycle_door_verb(verb: &str, spirit: Option<&str>, _color: ColorChoice) -> ExitCode {
    let Some(name) = spirit else {
        eprintln!("maosctl: {verb} requires a spirit argument, e.g. 'maosctl {verb} hello-spirit'");
        return ExitCode::from(2);
    };
    if let Some(code) = smoke_short_circuit(verb, name) {
        return code;
    }
    if let Some(code) = checked_id(verb, name) {
        return code;
    }
    door_verb(verb, |config| {
        door_client::exchange(
            config,
            "POST",
            &format!("/v1/spirits/{name}/{verb}"),
            None,
            DEFAULT_ROUTE_BUDGET,
        )
    })
}

/// `uninstall` — durable (AC5 / D-16-1-D): the door when it answers, the
/// offline child when nothing is configured or the connect is refused. The
/// preserved terminal contract 0/3/4/5 rides the door's `terminal_code`
/// body field; the offline child's exit codes forward unchanged.
fn dispatch_uninstall(args: &UninstallArgs, color: ColorChoice) -> ExitCode {
    let Some(name) = args.spirit.as_deref() else {
        eprintln!(
            "maosctl: uninstall requires a spirit argument, e.g. 'maosctl uninstall hello-spirit'"
        );
        return ExitCode::from(2);
    };
    if let Some(code) = smoke_short_circuit("uninstall", name) {
        return code;
    }
    if let Some(code) = checked_id("uninstall", name) {
        return code;
    }
    durable_verb(
        "uninstall",
        |config| {
            door_client::exchange(
                config,
                "POST",
                &format!("/v1/spirits/{name}/uninstall"),
                None,
                LONG_ROUTE_BUDGET,
            )
        },
        || uninstall_offline_arm(name, color),
    )
}

/// The offline uninstall one-shot (the surviving lifecycle spawn site): the
/// CHILD takes the exclusive store-lock set (500 ms retry) and prints its
/// own `maos: no daemon holds <paths>; running offline` line after
/// acquiring — never maosctl, which cannot know what the lock will decide.
fn uninstall_offline_arm(name: &str, color: ColorChoice) -> ExitCode {
    #[cfg(unix)]
    {
        let bin = maos_bin_path();
        let mut cmd = std::process::Command::new(&bin);
        cmd.env("MAOS_ONE_SHOT", "uninstall");
        cmd.env("MAOS_SPIRIT_ID", name);
        if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
            cmd.env("NO_COLOR", "1");
        }
        exec_and_forward(&mut cmd, &bin)
    }
    #[cfg(not(unix))]
    {
        let _ = (name, color);
        door_client::offline_unsupported("uninstall")
    }
}

fn release_pubkey() -> ExitCode {
    println!(
        "{}",
        hex::encode(maos_audit::release_verify::RELEASE_PUBKEY)
    );
    ExitCode::SUCCESS
}

/// `j1-crosshost-2e` AC2 (F1) — `maosctl cohort <sign>`.
fn dispatch_cohort(args: &crate::cli::CohortArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        crate::cli::CohortOp::Sign {
            manifest,
            authority_key,
            output,
        } => cohort_sign(manifest, authority_key, output.as_deref()),
    }
}

/// Sign a cohort manifest so a cohort daemon will boot.
///
/// Before this existed, `CohortManifest::signed_with` had ZERO non-test callers
/// (`crates/maos-cohort/src/manifest.rs:546`), while the daemon refuses to boot
/// without a manifest that verifies against its pinned authority keys. Host B of
/// a two-host run was therefore unreachable: a well-formed manifest with a
/// validly pinned key still died with
/// `EInvalidSignature("expected 64 bytes (128 hex chars), got 0 bytes")`.
///
/// Three refusals are deliberate and each closes a way of producing a manifest
/// that looks signed but is not trustworthy:
/// 1. `--authority-key` is explicit, so a cohort root can never silently become
///    the operator's audit root.
/// 2. The manifest must `parse_and_validate` against the authority keys IT
///    declares, so a structurally invalid manifest is refused before signing
///    rather than at a remote host's boot.
/// 3. The signer must actually BE a declared authority. `signed_with` does not
///    check this; without the check, this command would happily sign a manifest
///    naming somebody else as the authority — a forgery tool.
fn cohort_sign(
    manifest_path: &std::path::Path,
    authority_key_path: &std::path::Path,
    output: Option<&std::path::Path>,
) -> ExitCode {
    use ed25519_dalek::SigningKey;
    use maos_cohort::{CohortManifest, PinnedAuthorityKeys};

    macro_rules! fail {
        ($($arg:tt)*) => {{
            eprintln!("maosctl: cohort sign — {}", format!($($arg)*));
            return ExitCode::from(2);
        }};
    }

    // EXPLICIT path only. Passing `&None` here would honour MAOS_AUDIT_KEY and
    // the default audit root (`maos_domain::audit_key::load_audit_key_seed`),
    // welding the cohort trust root to the audit trust root.
    let seed = match maos_domain::audit_key::load_audit_key_seed(&Some(
        authority_key_path.to_path_buf(),
    )) {
        Ok(seed) => seed,
        Err(e) => fail!(
            "cannot load authority key {}: {e}",
            authority_key_path.display()
        ),
    };
    let signing_key = SigningKey::from_bytes(&seed);
    let signer_hex = hex::encode(signing_key.verifying_key().to_bytes());

    let text = match std::fs::read_to_string(manifest_path) {
        Ok(text) => text,
        Err(e) => fail!("cannot read manifest {}: {e}", manifest_path.display()),
    };

    // `signature` has no serde default, so an unsigned manifest cannot
    // deserialize as written. Supply an empty signature purely so the BODY can be
    // parsed and validated; the value is replaced by `signed_with` below and is
    // never trusted.
    let (parsed, normalized_for_body): (CohortManifest, String) = {
        let mut doc: toml::Value = match toml::from_str(&text) {
            Ok(doc) => doc,
            Err(e) => fail!(
                "manifest {} does not parse as TOML: {e}",
                manifest_path.display()
            ),
        };
        if let Some(table) = doc.as_table_mut() {
            table.insert(
                "signature".to_string(),
                toml::Value::Table(toml::map::Map::from_iter([(
                    "sig".to_string(),
                    toml::Value::String(String::new()),
                )])),
            );
        }
        let normalized = match toml::to_string(&doc) {
            Ok(normalized) => normalized,
            Err(e) => fail!("cannot normalize manifest for validation: {e}"),
        };
        match toml::from_str(&normalized) {
            Ok(parsed) => (parsed, normalized),
            Err(e) => fail!(
                "manifest {} is not a cohort manifest: {e}",
                manifest_path.display()
            ),
        }
    };

    // Validate the body against the authority set the manifest itself declares.
    let pinned = match PinnedAuthorityKeys::from_hex(&parsed.authority.keys) {
        Ok(pinned) => pinned,
        Err(e) => fail!("manifest declares an unusable authority key set: {e}"),
    };

    // The signer must be one of the declared authorities. `signed_with` will sign
    // anything; a tool that signs a manifest naming another authority is a
    // forgery tool, not a signer.
    if !parsed
        .authority
        .keys
        .iter()
        .any(|declared| declared.eq_ignore_ascii_case(&signer_hex))
    {
        fail!(
            "refusing to sign: the supplied key ({}…) is not among the manifest's declared \
             authority.keys. Signing a manifest that names a DIFFERENT authority would produce \
             an artifact whose signature and whose claimed signer disagree",
            &signer_hex[..16]
        );
    }

    // §A6 review P6 — validate the BODY before signing, per AC2.3's letter.
    // Previously this ran only on the signed artifact after `signed_with`;
    // the safety net was equivalent but the ordering deviated from the spec.
    if let Err(e) = CohortManifest::parse_and_validate(&normalized_for_body, &pinned) {
        fail!(
            "manifest {} does not validate: {e}",
            manifest_path.display()
        );
    }

    let signed = parsed.signed_with(&signing_key);

    // Prove the artifact we are about to emit actually verifies — STRUCTURALLY
    // (`parse_and_validate`) AND CRYPTOGRAPHICALLY (`verify_signature`).
    // §A6 review P6: the first version called only `parse_and_validate`, which
    // is structural by its own doc (`manifest.rs:355-356`); the signature's
    // round-trip through `toml::to_string` was never proven here. The daemon
    // verifies at boot, but a signer that discovers its own defect at a remote
    // host's boot instead of at the signing site is exactly what AC2 forbids.
    let serialized = match toml::to_string(&signed) {
        Ok(serialized) => serialized,
        Err(e) => fail!("cannot serialize the signed manifest: {e}"),
    };
    match CohortManifest::parse_and_validate(&serialized, &pinned) {
        Ok(reparsed) => {
            if let Err(e) = reparsed.verify_signature(&pinned) {
                fail!(
                    "the signed manifest's signature does not verify against its own declared \
                     authority: {e}"
                );
            }
        }
        Err(e) => {
            fail!("the signed manifest does not validate against its own declared authority: {e}")
        }
    }

    // §A6 review P7 — refuse an --output that aliases the authority key. The key
    // is already loaded by this point, so the write below would otherwise
    // silently replace the seed with manifest TOML and report success — a
    // deterministic destruction of a PUBLISHED cohort trust root (recovery
    // regenerates the root and invalidates PUBLISHED-FINGERPRINTS.md). Compare
    // by file identity, not string equality, so symlinks and hard links are
    // caught too.
    if let Some(path) = output {
        if same_file(path, authority_key_path) {
            fail!(
                "refusing to write: --output {} would overwrite --authority-key {}. The seed is \
                 loaded already, so this write would silently destroy the cohort trust root and \
                 report success. Write the signed manifest elsewhere.",
                path.display(),
                authority_key_path.display()
            );
        }
        if let Err(e) = std::fs::write(path, &serialized) {
            fail!("cannot write {}: {e}", path.display());
        }
        eprintln!(
            "maosctl: cohort sign — signed {} (cohort `{}` v{}, {} member(s)) under authority {} \
             → {}",
            manifest_path.display(),
            signed.cohort_id,
            signed.version,
            signed.members.len(),
            signer_hex,
            path.display()
        );
    } else {
        print!("{serialized}");
        eprintln!(
            "maosctl: cohort sign — signed {} (cohort `{}` v{}, {} member(s)) under authority {} \
             → stdout",
            manifest_path.display(),
            signed.cohort_id,
            signed.version,
            signed.members.len(),
            signer_hex
        );
    }
    ExitCode::SUCCESS
}

/// §A6 review P7 — same-file check for `cohort sign`'s `--output` vs
/// `--authority-key`. Two paths denote the same file when they canonicalize to
/// the same path, or when one does not exist yet but its parent canonicalizes
/// identically and the file names match (covers `./k` vs `k` for a not-yet-born
/// output). Existing symlinks/hard links are caught by `same_file`-style dev+ino
/// comparison; a symlink whose TARGET does not exist yet falls back to the
/// canonical-parent comparison. Not a general-purpose utility: callers pass one
/// existing path (the key) and one possibly-absent path (the output).
fn same_file(output: &std::path::Path, key: &std::path::Path) -> bool {
    let meta = |p: &std::path::Path| std::fs::metadata(p).ok();
    if let (Some(a), Some(b)) = (meta(output), meta(key)) {
        // Symlinks are followed by `metadata`; hard links share dev+ino.
        // Windows has no stable dev+ino through std, so canonicalize both
        // existing paths instead (symlinks still resolve; hard-link aliases
        // are the accepted gap — std cannot see them without
        // GetFileInformationByHandle, and the not-yet-born fallback below
        // has the same gap).
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            return a.dev() == b.dev() && a.ino() == b.ino();
        }
        #[cfg(not(unix))]
        {
            let _ = (&a, &b);
            return match (output.canonicalize().ok(), key.canonicalize().ok()) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            };
        }
    }
    // Output does not exist yet (the normal case): compare the canonical parent
    // plus the final component, so `dir/../key`, `./key` and `key` all alias.
    let canon = |p: &std::path::Path| {
        let parent = p.parent()?;
        let canonical_parent = std::fs::canonicalize(parent).ok()?;
        Some(canonical_parent.join(p.file_name()?))
    };
    canon(output) == canon(key)
}

/// Story 9.4 AC-3 — `maosctl backup <create|verify|restore>`.
fn dispatch_backup(args: &BackupArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        BackupOp::Create { dest } => {
            let source = default_transparency_log_path();
            let dest_path = std::path::Path::new(dest);
            match crate::backup::backup_transparency_log(&source, dest_path) {
                Ok(()) => {
                    eprintln!("backup created: {dest}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: backup failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        BackupOp::Verify { backup } => {
            let source = default_transparency_log_path();
            let backup_path = std::path::Path::new(backup);
            match verify_backup_via_cold_restore(&source, backup_path) {
                Ok(()) => {
                    eprintln!("backup integrity verified: cold-restore Merkle roots match");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: backup verification failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        BackupOp::Restore { backup, target } => {
            let start = std::time::Instant::now();
            let backup_path = std::path::Path::new(backup);
            let target_path = std::path::Path::new(target);
            // Restore = copy backup to target via the same backup API.
            match crate::backup::backup_transparency_log(backup_path, target_path) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("error: restore failed: {e}");
                    return ExitCode::FAILURE;
                }
            }
            // Verify the restored copy against the backup via cold restore.
            match verify_backup_via_cold_restore(backup_path, target_path) {
                Ok(()) => {
                    let elapsed = start.elapsed();
                    eprintln!(
                        "restore complete: {target} (Merkle verified, RTO={:.3}s)",
                        elapsed.as_secs_f64()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: restored copy verification failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// R-DR1 — verify backup integrity by performing an independent cold restore.
///
/// Restores `backup_path` to a temporary file, then compares the Merkle root of
/// that restored copy against the live source TL. This proves the backup is not
/// only readable but restorable to a new database.
fn verify_backup_via_cold_restore(
    source_path: &std::path::Path,
    backup_path: &std::path::Path,
) -> Result<(), String> {
    let restored = crate::backup::cold_restore_to_temp(backup_path)
        .map_err(|e| format!("cold restore failed: {e}"))?;
    let source_root = maos_audit::backup::compute_merkle_root(source_path)
        .map_err(|e| format!("source Merkle root failed: {e}"))?;
    let restored_root = maos_audit::backup::compute_merkle_root(&restored)
        .map_err(|e| format!("restored Merkle root failed: {e}"))?;
    if source_root != restored_root {
        return Err(format!(
            "Merkle root mismatch: source={}, restored={}",
            hex::encode(source_root),
            hex::encode(restored_root)
        ));
    }
    Ok(())
}

/// Story 10.4a AC2 (NFR-Ops-10) — `maosctl migrate sqlite-to-postgres`.
///
/// Drives the triple-oracle migration engine in `maos-loom-lite`. The SQLite
/// source MUST be quiesced (no active writers) before invocation. The engine:
///   1. Reads all frames from SQLite (canonical serialization).
///   2. Computes source oracles (Merkle root, payload oracle, row count).
///   3. Creates the Postgres TL schema.
///   4. Inserts in batches of 10 000 (multiple batch boundaries for proven-red).
///   5. Independently re-derives target oracles from Postgres.
///   6. Verifies all three oracles pass.
///
/// On verification failure with `--rollback-on-failure` (default), the Postgres
/// target table is dropped and the SQLite source is verified intact.
fn dispatch_migrate(args: &MigrateArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        MigrateOp::SqliteToPostgres {
            from,
            to,
            rollback_on_failure,
        } => {
            let sqlite_path = std::path::Path::new(from);
            if !sqlite_path.exists() {
                eprintln!("error: source SQLite database not found: {from}");
                return ExitCode::FAILURE;
            }

            // P14 — sslmode guard.  `maos migrate` connects with `NoTls`, which
            // cannot honour an encryption request.  An operator passing
            // sslmode=require/verify-ca/verify-full/prefer would otherwise get a
            // silent cleartext downgrade of credentials + payloads.  Mirrors the
            // guard in `maos_loom_lite::store` (`StoreConfig`); refuses rather
            // than silently send plaintext over an unencrypted link.
            if requests_tls_unsupported_by_notls(to) {
                eprintln!(
                    "error: '{to}' requests sslmode=require/verify-ca/verify-full/prefer, \
                     but `maos migrate` ships NoTls-only. Set sslmode=disable (e.g. loopback) \
                     or front Postgres with a TLS-terminating sidecar. Refusing to send \
                     plaintext credentials over an unencrypted link."
                );
                return ExitCode::FAILURE;
            }

            // The migration engine is async (tokio-postgres). We need a runtime
            // to drive it from the sync CLI entry point.
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("error: failed to create async runtime: {e}");
                    return ExitCode::FAILURE;
                }
            };

            runtime.block_on(async move {
                // Capture the pre-migration source root BEFORE any target write
                // (B13 — non-tautological rollback snapshot).
                let pre_source_root = match maos_audit::backup::compute_merkle_root(sqlite_path) {
                    Ok(root) => root,
                    Err(e) => {
                        eprintln!("error: cannot read pre-migration source root: {e}");
                        return ExitCode::FAILURE;
                    }
                };

                // Connect to Postgres with a bounded connect timeout (B19 — an
                // unreachable host must not hang the CLI forever).
                let connect_fut = tokio_postgres::connect(to, tokio_postgres::NoTls);
                let (mut client, connection) =
                    match tokio::time::timeout(std::time::Duration::from_secs(30), connect_fut)
                        .await
                    {
                        Ok(Ok(c)) => c,
                        Ok(Err(e)) => {
                            eprintln!("error: failed to connect to Postgres: {e}");
                            return ExitCode::FAILURE;
                        }
                        Err(_) => {
                            eprintln!("error: timed out connecting to Postgres after 30s");
                            return ExitCode::FAILURE;
                        }
                    };
                tokio::spawn(async move {
                    if let Err(e) = connection.await {
                        eprintln!("warn: postgres connection error: {e}");
                    }
                });
                // Bound per-statement execution too (B19).
                if let Err(e) = client.batch_execute("SET statement_timeout = 60000").await {
                    eprintln!("error: failed to set statement_timeout: {e}");
                    return ExitCode::FAILURE;
                }

                // Run the forward migration (transactional, triple-oracle verified).
                match maos_loom_lite::migration::migrate_sqlite_to_postgres(
                    sqlite_path,
                    &mut client,
                )
                .await
                {
                    Ok(result) => {
                        eprintln!(
                            "migration complete: {} rows transferred ({} batches)",
                            result.target_row_count,
                            (result.target_row_count as usize)
                                .div_ceil(maos_loom_lite::migration::BATCH_SIZE)
                        );
                        eprintln!(
                            "  source merkle root:    {}",
                            hex::encode(result.source_merkle_root)
                        );
                        eprintln!(
                            "  target merkle root:    {}",
                            hex::encode(result.target_merkle_root)
                        );
                        eprintln!(
                            "  source payload oracle: {}",
                            hex::encode(result.source_payload_oracle)
                        );
                        eprintln!(
                            "  target payload oracle: {}",
                            hex::encode(result.target_payload_oracle)
                        );
                        eprintln!("triple-oracle verification: PASS");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        eprintln!("error: migration verification failed: {e}");
                        if *rollback_on_failure {
                            // Rollback using the PRE-migration snapshot (B13).
                            match maos_loom_lite::migration::rollback_migration(
                                sqlite_path,
                                &client,
                                pre_source_root,
                            )
                            .await
                            {
                                Ok(()) => {
                                    eprintln!(
                                        "rollback complete: Postgres target dropped, \
                                         SQLite source verified intact (pre-migration root matched)"
                                    );
                                }
                                Err(re) => {
                                    eprintln!("error: rollback failed: {re}");
                                }
                            }
                        }
                        ExitCode::FAILURE
                    }
                }
            })
        }
    }
}

/// P14 — detect an `sslmode` that requests encryption `NoTls` cannot provide.
///
/// `maos migrate` connects with `tokio_postgres::NoTls` directly (it does not
/// route through the Loom-lite deadpool, so the store-level guard does not
/// apply).  An operator EXPLICITLY requesting `sslmode=require`/`verify-ca`/
/// `verify-full`/`prefer` must be refused rather than silently downgraded to a
/// cleartext connection that leaks credentials + payloads.  Detected at the
/// string level (mirroring `maos_loom_lite::store`) because the CLI uses `NoTls`
/// and never parses the connection string's sslmode.
fn requests_tls_unsupported_by_notls(conn_str: &str) -> bool {
    conn_str.to_lowercase().split_whitespace().any(|kv| {
        kv.strip_prefix("sslmode=")
            .is_some_and(|v| matches!(v, "require" | "verify-ca" | "verify-full" | "prefer"))
    })
}

/// Story 9.3b — `maosctl governance admit`: a durable verb (D-16-1-A) — the
/// door when it answers, the one-shot env channel otherwise (the child takes
/// the exclusive store-lock set; the kernel-side handler writes the
/// schema-lifecycle registry row + governance event frame either way).
fn dispatch_governance(args: &GovernanceArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        GovernanceOp::Admit {
            schema_id,
            version,
            content_hash,
            supersedes,
            ratified_by,
            effective_at_ns,
        } => {
            // Over the door the WHOLE admission request travels as one JSON
            // value (`AdmitGovernanceSchema{schema}`) — the daemon must not
            // re-read operator intent from its own environment.
            let body = serde_json::to_vec(&serde_json::json!({
                "schema_id": schema_id,
                "version": version,
                "content_hash": content_hash,
                "supersedes": supersedes,
                "ratified_by": ratified_by,
                "effective_at_ns": effective_at_ns,
            }))
            .expect("serializing a hand-built Value cannot fail");
            durable_verb(
                "governance admit",
                |config| {
                    door_client::exchange(
                        config,
                        "POST",
                        "/v1/governance/schemas",
                        Some(("application/json", &body)),
                        DEFAULT_ROUTE_BUDGET,
                    )
                },
                || governance_offline_arm(args, color),
            )
        }
    }
}

/// The offline governance one-shot: env channel unchanged from Story 9.3b.
fn governance_offline_arm(args: &GovernanceArgs, color: ColorChoice) -> ExitCode {
    #[cfg(unix)]
    {
        let GovernanceOp::Admit {
            schema_id,
            version,
            content_hash,
            supersedes,
            ratified_by,
            effective_at_ns,
        } = &args.op;
        let bin = maos_bin_path();
        let mut cmd = std::process::Command::new(&bin);
        cmd.env("MAOS_ONE_SHOT", "governance-admit");
        cmd.env("MAOS_GOVERNANCE_SCHEMA_ID", schema_id);
        cmd.env("MAOS_GOVERNANCE_VERSION", version.to_string());
        cmd.env("MAOS_GOVERNANCE_CONTENT_HASH", content_hash);
        cmd.env("MAOS_GOVERNANCE_RATIFIED_BY", ratified_by);
        cmd.env(
            "MAOS_GOVERNANCE_EFFECTIVE_AT_NS",
            effective_at_ns.to_string(),
        );
        if let Some(s) = supersedes {
            cmd.env("MAOS_GOVERNANCE_SUPERSEDES", s);
        }
        if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
            cmd.env("NO_COLOR", "1");
        }
        exec_and_forward(&mut cmd, &bin)
    }
    #[cfg(not(unix))]
    {
        let _ = (args, color);
        door_client::offline_unsupported("governance admit")
    }
}

/// Story 9.7 (FR39) — `maosctl skills <list|approve|reject>`.
///
/// `list` runs filesystem discovery over the conventional `[skills.search_path]`
/// roots, then derives each skill's admission state from the Transparency Log
/// decided-set (AC-4: pending = discovered MINUS decided). `approve`/`reject`
/// journal the operator decision to the TL FIRST (the commit point), then
/// update the `queue.json` cache. `queue.json` is a rebuildable cache over the
/// append-only TL — not the source of truth.
fn dispatch_skills(args: &SkillsArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        SkillsOp::List { root } => dispatch_skills_list(root),
        SkillsOp::Approve { skill_id, actor } => {
            dispatch_skills_decide(skill_id, true, actor.as_deref())
        }
        SkillsOp::Reject { skill_id, actor } => {
            dispatch_skills_decide(skill_id, false, actor.as_deref())
        }
    }
}

/// Resolve the operator identity for an approve/reject (AC-3 `actor`).
/// Precedence: explicit `--actor` flag → `$USER` → `"operator"` fallback.
fn resolve_actor(actor: Option<&str>) -> String {
    actor
        .map(str::to_string)
        .or_else(|| std::env::var("USER").ok().filter(|u| !u.is_empty()))
        .unwrap_or_else(|| "operator".to_string())
}

/// Load the skill-queue cache, warning appropriately when it is absent,
/// future-schema, or corrupt. The cache is a rebuildable projection over the
/// Transparency Log (F3/AC-4); a bad cache must never brick the CLI surface.
fn load_cache_warn(store: &maos_skill::LocalFsSkillQueueStore) -> Vec<maos_skill::QueueEntry> {
    use maos_skill::ESkillStore;
    match store.load() {
        Ok(s) => s,
        Err(ESkillStore::UnknownSchemaVersion(v)) => {
            eprintln!(
                "maosctl skills: warning: queue cache schema `{v}` is newer/unknown — rebuilding from discovery + TL"
            );
            Vec::new()
        }
        Err(ESkillStore::Io(e)) => {
            eprintln!(
                "maosctl skills: warning: queue cache I/O error ({e}) — proceeding from discovery + TL"
            );
            Vec::new()
        }
        Err(ESkillStore::Json(e)) => {
            eprintln!(
                "maosctl skills: warning: queue cache JSON error ({e}) — proceeding from discovery + TL"
            );
            Vec::new()
        }
        Err(_) => {
            eprintln!(
                "maosctl skills: warning: queue cache unreadable — proceeding from discovery + TL"
            );
            Vec::new()
        }
    }
}

/// `maosctl skills list` — discover + derive admission state from the TL.
fn dispatch_skills_list(root: &[String]) -> ExitCode {
    let roots: Vec<PathBuf> = if root.is_empty() {
        maos_skill::default_search_path()
    } else {
        root.iter().map(PathBuf::from).collect()
    };
    let outcome = maos_skill::discover_skills_detailed(&roots);
    let store = maos_skill::LocalFsSkillQueueStore::new();
    let stored = load_cache_warn(&store);
    let tl_path = default_transparency_log_path();
    let view = admission_view(&outcome.discovered, stored, &tl_path);
    if !view.tl_readable {
        eprintln!(
            "maosctl skills: warning: Transparency Log unreadable at {} — showing cache/discovery state (TL reconcile skipped)",
            tl_path.display()
        );
    }
    let state_by_id: HashMap<SkillId, SkillAdmissionState> = view
        .entries
        .iter()
        .map(|e| (e.id.clone(), e.state))
        .collect();
    if outcome.discovered.is_empty() && outcome.skipped.is_empty() && view.entries.is_empty() {
        println!("maosctl skills: no skills discovered on the search path");
    }
    for d in &outcome.discovered {
        let state = state_by_id
            .get(&d.skill.manifest.skill_id())
            .copied()
            .unwrap_or(d.state);
        println!(
            "{:<24} {:<10} {:?}  ({})",
            d.skill.manifest.id,
            d.skill.manifest.version,
            state,
            d.source_path.display()
        );
    }
    // Cache-only entries (previously discovered, now off the search path).
    let on_path: HashSet<SkillId> = outcome
        .discovered
        .iter()
        .map(|d| d.skill.manifest.skill_id())
        .collect();
    for e in &view.entries {
        if !on_path.contains(&e.id) {
            println!(
                "{:<24} {:<10} {:?}  (queued, not on search path)",
                e.id, e.version, e.state
            );
        }
    }
    for (path, reason) in &outcome.skipped {
        eprintln!("maosctl skills: skipped {} — {}", path.display(), reason);
    }
    ExitCode::SUCCESS
}

/// `maosctl skills approve/reject` — journal-FIRST to the TL, then update cache.
///
/// Story 9.7 AC-3 (R2 journal-first ordering): the TL write is the COMMIT
/// POINT. Only on a committed journal row is the `queue.json` cache rewritten.
/// If the TL write fails the command aborts, mutates NOTHING, and reports
/// failure with a non-zero exit — never silent-success-without-journal.
fn dispatch_skills_decide(skill_id: &str, approve: bool, actor: Option<&str>) -> ExitCode {
    let store = maos_skill::LocalFsSkillQueueStore::new();
    let roots = maos_skill::default_search_path();
    let tl_path = default_transparency_log_path();
    let actor = resolve_actor(actor);
    let verb = if approve { "approve" } else { "reject" };

    let outcome = maos_skill::discover_skills_detailed(&roots);
    let stored = load_cache_warn(&store);
    match decide_skill(
        &outcome.discovered,
        stored,
        &store,
        &tl_path,
        skill_id,
        approve,
        &actor,
    ) {
        DecideOutcome::Applied { new_state } => {
            println!("maosctl skills: skill `{skill_id}` {verb}d by `{actor}` — now {new_state:?}");
            ExitCode::SUCCESS
        }
        DecideOutcome::AlreadyResolved { state } => {
            eprintln!("maosctl skills: skill `{skill_id}` is already {state:?} — no action taken");
            ExitCode::SUCCESS
        }
        DecideOutcome::NotFound => {
            eprintln!(
                "maosctl skills: skill `{skill_id}` not found on the search path — use `maosctl skills list` to see available skills"
            );
            ExitCode::FAILURE
        }
        DecideOutcome::JournalFailed(e) => {
            eprintln!(
                "maosctl skills: FAILED to journal {verb} decision to Transparency Log: {e}\n\
                 Decision NOT applied — no silent loss. Retry when the TL is accessible."
            );
            ExitCode::FAILURE
        }
    }
}

// ─── Story 9.7 AC-4: admission view + reconcile (pub for tests) ────────

/// The TL decided-set: `target -> is_approve`. Built from `query_approvals`
/// (ordered `decision_id ASC` = monotonic insertion order); the LAST row per
/// target wins (LWW by `decision_id`, NOT the non-monotonic `timestamp_ns` —
/// Review #5). Only targets whose latest row is approve/reject are kept;
/// enqueue rows + unrelated capabilities are filtered out (R5).
pub fn decided_set(approvals: &[ApprovalDecision]) -> HashMap<String, bool> {
    let mut latest_per_target: HashMap<&str, &ApprovalDecision> = HashMap::new();
    for d in approvals {
        latest_per_target.insert(d.target.as_str(), d);
    }
    latest_per_target
        .into_iter()
        .filter_map(|(target, d)| match d.capability.as_str() {
            "skill.admission.approve" => Some((target.to_string(), true)),
            "skill.admission.reject" => Some((target.to_string(), false)),
            _ => None,
        })
        .collect()
}

/// Derive an entry's admission state from the decided-set (AC-4). A target not
/// in the decided-set is Pending — this is the demote path: a re-enqueue makes
/// the latest TL row an enqueue (not approve/reject), so the skill returns to
/// Pending. (Review #4.)
fn derive_state(target: &str, decided: &HashMap<String, bool>) -> SkillAdmissionState {
    match decided.get(target) {
        Some(true) => SkillAdmissionState::Admitted,
        Some(false) => SkillAdmissionState::Rejected,
        None => SkillAdmissionState::Pending,
    }
}

/// Reconcile a set of queue entries against the TL decided-set (AC-4). Each
/// entry's state is derived fresh from the TL. Pure + testable — the reconcile
/// tests call THIS, not a hand-mirrored copy (Review #10).
pub fn reconcile_entries(
    entries: Vec<maos_skill::QueueEntry>,
    decided: &HashMap<String, bool>,
) -> Vec<maos_skill::QueueEntry> {
    entries
        .into_iter()
        .map(|mut e| {
            let target = maos_skill::approval_target::approval_target(&e.id, &e.version);
            e.state = derive_state(&target, decided);
            e
        })
        .collect()
}

fn load_decided_set(tl_path: &std::path::Path) -> Result<HashMap<String, bool>, String> {
    let tl = TransparencyLogAdapter::open(tl_path, 0).map_err(|e| e.to_string())?;
    let approvals = tl.query_approvals(None).map_err(|e| e.to_string())?;
    Ok(decided_set(&approvals))
}

/// The reconciled admission view (AC-4). `entries` = discovered skills (state
/// derived from the TL) + stored entries no longer on the search path. The TL
/// is the source of truth; `queue.json` is a rebuildable cache.
pub struct AdmissionView {
    pub entries: Vec<maos_skill::QueueEntry>,
    pub tl_readable: bool,
}

/// Compute the admission view from discovery + cache + TL (AC-4). Discovered
/// skills default to Pending, then every entry's state is derived from the TL
/// decided-set. If the TL is unreadable, the cache/discovery state is kept and
/// `tl_readable=false` (the caller warns — Review #7).
pub fn admission_view(
    discovered: &[DiscoveredSkill],
    stored: Vec<maos_skill::QueueEntry>,
    tl_path: &std::path::Path,
) -> AdmissionView {
    let mut entries: Vec<maos_skill::QueueEntry> = discovered
        .iter()
        .map(|d| maos_skill::QueueEntry {
            id: d.skill.manifest.skill_id(),
            version: d.skill.manifest.skill_version(),
            // 9.7 boundary: filesystem discovery has no provenance signal.
            // `AuthorSelf` and `RevisionProposal` arise only from enqueue-time
            // paths (daemon/kernel); for the CLI search path the skill is
            // package-shipped by construction. Faithful provenance fidelity is
            // coupled to v2 schema + Epic-10 F6b/R8 daemon-enqueue work.
            entry_path: "package_shipped".to_string(),
            state: SkillAdmissionState::Pending,
        })
        .collect();
    let on_path: HashSet<SkillId> = discovered
        .iter()
        .map(|d| d.skill.manifest.skill_id())
        .collect();
    for e in stored {
        if !on_path.contains(&e.id) {
            entries.push(e);
        }
    }
    match load_decided_set(tl_path) {
        Ok(decided) => AdmissionView {
            entries: reconcile_entries(entries, &decided),
            tl_readable: true,
        },
        Err(_) => AdmissionView {
            entries,
            tl_readable: false,
        },
    }
}

/// The typed outcome of an approve/reject (testable; no `ExitCode`).
#[derive(Debug)]
pub enum DecideOutcome {
    /// Pending -> Admitted/Rejected: journaled to the TL + cache rewritten.
    Applied { new_state: SkillAdmissionState },
    /// Already Admitted/Rejected — no-op (AC-2).
    AlreadyResolved { state: SkillAdmissionState },
    /// Neither discovered nor in the cache.
    NotFound,
    /// TL journal write failed — NOTHING mutated (no silent loss; AC-3/R2).
    JournalFailed(String),
}

/// Journal-FIRST decide core (AC-3/R2). Loads the admission view, validates the
/// transition, journals the decision to the TL (the commit point), and ONLY on
/// success rewrites the `queue.json` cache. Testable via explicit params
/// (discovered + stored + tl_path — no env vars). Persisting the view directly
/// (not via the lossy in-mem enum round-trip) preserves entry_path labels
/// (Review #12) and makes a discovered skill approvable (Review D1 Critical).
pub fn decide_skill(
    discovered: &[DiscoveredSkill],
    stored: Vec<maos_skill::QueueEntry>,
    store: &maos_skill::LocalFsSkillQueueStore,
    tl_path: &std::path::Path,
    skill_id: &str,
    approve: bool,
    actor: &str,
) -> DecideOutcome {
    let id = SkillId::from(skill_id);
    let mut view = admission_view(discovered, stored, tl_path);

    let Some(idx) = view.entries.iter().position(|e| e.id == id) else {
        return DecideOutcome::NotFound;
    };
    let state_before = view.entries[idx].state;
    if state_before != SkillAdmissionState::Pending {
        return DecideOutcome::AlreadyResolved {
            state: state_before,
        };
    }

    let verb = if approve { "approve" } else { "reject" };
    let capability = if approve {
        "skill.admission.approve"
    } else {
        "skill.admission.reject"
    };
    let target = maos_skill::approval_target::approval_target(
        &view.entries[idx].id,
        &view.entries[idx].version,
    );
    let decision = ApprovalDecision {
        actor: actor.to_string(),
        target,
        capability: capability.to_string(),
        intent: "cli_operator_decision".to_string(),
        decision: approve,
        reasoning: Some(format!(
            "operator {actor} {verb}d skill `{skill_id}` via maosctl skills {verb}"
        )),
    };

    // Journal-FIRST (R2): the TL write is the commit point.
    let tl = match TransparencyLogAdapter::open(tl_path, 0) {
        Ok(tl) => tl,
        Err(e) => {
            return DecideOutcome::JournalFailed(format!("open TL at {}: {e}", tl_path.display()))
        }
    };
    if let Err(e) = tl.insert_approval_decision(decision) {
        return DecideOutcome::JournalFailed(format!("insert_approval_decision: {e}"));
    }

    // Only on a committed journal row: update the cache + persist.
    let new_state = if approve {
        SkillAdmissionState::Admitted
    } else {
        SkillAdmissionState::Rejected
    };
    view.entries[idx].state = new_state;
    // Best-effort cache write — the TL is the source of truth and reconcile
    // recovers on the next load.
    let _ = store.save(&view.entries);
    DecideOutcome::Applied { new_state }
}

fn dispatch_import(args: &ImportArgs, _color: ColorChoice) -> ExitCode {
    // TODO: registry_uri is parsed by clap but not yet wired to storage location.
    // The LocalFsRegistryStorage uses a fixed default path; custom URI override
    // is deferred to v0.7+. Log a warning if the user provided one.
    if args.registry_uri.is_some() {
        eprintln!("maosctl import: warning: --registry-uri is not yet implemented ( Story 7.2 v1.0); ignored");
    }
    use maos_registry::admission::{admit_spirit, admit_spirit_with_attestation, AdmissionConfig};
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
            "public_vetted" => TrustTier::PublicVetted,
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

    let vetting = match (&args.vetting_attestation, &args.vetter_keyring) {
        (None, None) => None,
        (Some(attestation_path), Some(keyring_path)) => {
            let attestation_bytes = match std::fs::read(attestation_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!(
                        "maosctl import: failed to read vetting attestation {}: {error}",
                        attestation_path.display()
                    );
                    return ExitCode::from(1);
                }
            };
            let attestation = match serde_cbor::from_slice::<maos_compliance::VettingAttestation>(
                &attestation_bytes,
            ) {
                Ok(attestation) => attestation,
                Err(error) => {
                    eprintln!("maosctl import: malformed vetting attestation: {error}");
                    return ExitCode::from(1);
                }
            };
            let keyring_bytes = match std::fs::read(keyring_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!(
                        "maosctl import: failed to read vetter keyring {}: {error}",
                        keyring_path.display()
                    );
                    return ExitCode::from(1);
                }
            };
            let keyring =
                match serde_cbor::from_slice::<maos_compliance::VetterKeyring>(&keyring_bytes) {
                    Ok(keyring) => keyring,
                    Err(error) => {
                        eprintln!("maosctl import: malformed vetter keyring: {error}");
                        return ExitCode::from(1);
                    }
                };
            let operator_seed = match maos_domain::audit_key::load_audit_key_seed(&None) {
                Ok(seed) => seed,
                Err(error) => {
                    eprintln!("maosctl import: operator audit root unavailable: {error}");
                    return ExitCode::from(1);
                }
            };
            let operator_root = maos_domain::audit_key::derive_audit_public_key(&operator_seed);
            Some((attestation, keyring, operator_root))
        }
        _ => {
            eprintln!(
                "maosctl import: --vetting-attestation and --vetter-keyring must be supplied together"
            );
            return ExitCode::from(2);
        }
    };

    // 4. Admit the Spirit.
    let admission_result = if let Some((attestation, keyring, operator_root)) = &vetting {
        let now_unix_ms = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_millis() as u64,
            Err(error) => {
                eprintln!("maosctl import: system clock precedes Unix epoch: {error}");
                return ExitCode::from(1);
            }
        };
        admit_spirit_with_attestation(
            &bundle.signed_package,
            &op_cfg,
            Some(attestation),
            keyring,
            operator_root,
            now_unix_ms,
        )
    } else {
        admit_spirit(&bundle.signed_package, &op_cfg)
    };
    let decision = match admission_result {
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
    let summary_str = match serde_json::to_string(&summary) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl import: failed to serialize summary: {e}");
            "{}".into()
        }
    };
    println!("{}", summary_str);
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

    // Accessibility smoke path (Story 1b.5c): unit tests assert the ANSI-free
    // cascade without spawning the full composition root, which keeps the test
    // suite fast and avoids pipe-deadlock hazards when the harness captures
    // stdout/stderr.
    if std::env::var_os("MAOS_ACCESSIBILITY_SMOKE").is_some() {
        eprintln!("maosctl: run smoke ok for {spirit}");
        return ExitCode::SUCCESS;
    }

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

    exec_and_forward(&mut cmd, &bin)
}

fn install(args: &InstallArgs, _color: ColorChoice) -> ExitCode {
    // Resolve pubkey: --release-pubkey override or bundled default.
    let pubkey: [u8; 32] = if let Some(hex_str) = &args.release_pubkey {
        let bytes = match hex::decode(hex_str) {
            Ok(b) if b.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&b);
                arr
            }
            Ok(b) => {
                eprintln!(
                    "maosctl: --release-pubkey must be 32 bytes (64 hex chars), got {} bytes",
                    b.len()
                );
                return ExitCode::from(2);
            }
            Err(e) => {
                eprintln!("maosctl: --release-pubkey invalid hex: {e}");
                return ExitCode::from(2);
            }
        };
        bytes
    } else {
        maos_audit::release_verify::RELEASE_PUBKEY
    };

    // Path B: local release artifact verification/install.
    if let Some(dir) = &args.from_local {
        return install_from_local(dir, &pubkey, args.verify_only, args.prefix.as_deref());
    }

    // Path A: legacy spirit install. The remote-fetch path is intentionally
    // removed at v0.5 (AC-1 scoped to --from-local); a 'v...' source string
    // no longer misroutes here.
    install_spirit(args)
}

/// Detect the release binary name for the current platform.
fn platform_binary_name() -> Result<&'static str, String> {
    if cfg!(target_arch = "x86_64") && cfg!(target_os = "linux") {
        Ok("maos-linux-amd64")
    } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "linux") {
        Ok("maos-linux-arm64")
    } else if cfg!(target_arch = "aarch64") && cfg!(target_os = "macos") {
        Ok("maos-darwin-arm64")
    } else if cfg!(target_arch = "x86_64") && cfg!(target_os = "windows") {
        Ok("maos-windows-amd64.exe")
    } else {
        Err(format!(
            "unsupported platform for release install: {}-{}",
            std::env::consts::ARCH,
            std::env::consts::OS
        ))
    }
}

/// Path B: verify (and optionally install) a locally-staged release artifact.
fn install_from_local(
    dir: &str,
    pubkey: &[u8; 32],
    verify_only: bool,
    prefix: Option<&std::path::Path>,
) -> ExitCode {
    let dir_path = std::path::Path::new(dir);
    let sums_path = dir_path.join("SHA256SUMS");
    let sig_path = dir_path.join("SHA256SUMS.sig");
    let binary_name = match platform_binary_name() {
        Ok(name) => name,
        Err(e) => {
            eprintln!("maosctl: {e}");
            return ExitCode::from(2);
        }
    };
    let bin_path = dir_path.join(binary_name);

    // Read SHA256SUMS
    let sums_content = match std::fs::read(&sums_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("maosctl: cannot read {}: {e}", sums_path.display());
            return ExitCode::from(2);
        }
    };

    // Read SHA256SUMS.sig (raw 64-byte Ed25519 signature)
    let sig_bytes: [u8; 64] = match std::fs::read(&sig_path) {
        Ok(b) if b.len() == 64 => {
            let mut arr = [0u8; 64];
            arr.copy_from_slice(&b);
            arr
        }
        Ok(b) => {
            eprintln!(
                "maosctl: {} must be 64 bytes (raw Ed25519 signature), got {} bytes",
                sig_path.display(),
                b.len()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: cannot read {}: {e}", sig_path.display());
            return ExitCode::from(2);
        }
    };

    // Read the binary
    let bin_content = match std::fs::read(&bin_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("maosctl: cannot read {}: {e}", bin_path.display());
            return ExitCode::from(2);
        }
    };

    // Full verification pipeline: signature → SHA256. Single-platform subset
    // verification is allowed because the operator staged only this artifact.
    let files: Vec<(&str, &[u8])> = vec![(binary_name, bin_content.as_slice())];
    match maos_audit::release_verify::verify_release(
        &sums_content,
        &sig_bytes,
        pubkey,
        &files,
        true,
    ) {
        Ok(entries) => {
            eprintln!("maosctl: verification passed for {} file(s)", entries.len());
            for entry in &entries {
                eprintln!("  ✓ {} ({})", entry.filename, &entry.hash[..16]);
            }
        }
        Err(e) => {
            eprintln!("maosctl: release verification FAILED: {e}");
            return ExitCode::from(1);
        }
    }

    if verify_only {
        return ExitCode::SUCCESS;
    }

    // Install: copy verified binary to the requested or default location.
    let install_target = if let Some(p) = prefix {
        p.join("maos")
    } else {
        match std::env::current_exe() {
            Ok(exe) => exe
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("maos"),
            Err(_) => std::path::PathBuf::from("/usr/local/bin/maos"),
        }
    };

    if let Some(parent) = install_target.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!(
                "maosctl: failed to create install directory {}: {e}",
                parent.display()
            );
            return ExitCode::from(2);
        }
    }

    match std::fs::copy(&bin_path, &install_target) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&install_target)
                    .map(|m| m.permissions())
                    .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o755));
                perms.set_mode(perms.mode() | 0o111); // ensure owner executable bit
                if let Err(e) = std::fs::set_permissions(&install_target, perms) {
                    eprintln!(
                        "maosctl: installed binary but failed to set executable permissions on {}: {e}",
                        install_target.display()
                    );
                    return ExitCode::from(2);
                }
            }
            eprintln!(
                "maosctl: installed verified binary to {}",
                install_target.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!(
                "maosctl: failed to install binary to {}: {e}",
                install_target.display()
            );
            ExitCode::from(2)
        }
    }
}

/// Path A: legacy spirit install (v0.1-α cargo build).
fn install_spirit(args: &InstallArgs) -> ExitCode {
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
    // keep under 60s.
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

/// Story 13.5b — host-global legal-hold operator surface (Story 16-1: the
/// `list` half is an IN-PROCESS durable reader (D-16-1-A); `release` is a
/// durable verb — door when it answers, the one-shot child otherwise).
fn dispatch_legal_hold(args: &LegalHoldArgs, color: ColorChoice) -> ExitCode {
    match &args.op {
        LegalHoldOp::List => legal_hold_list_reader(),
        LegalHoldOp::Release { principal } => {
            let principal = principal.trim();
            if principal.is_empty() {
                eprintln!("maosctl: legal-hold release requires a non-empty --principal");
                return ExitCode::from(2);
            }
            let body = serde_json::to_vec(&serde_json::json!({ "principal": principal })) // xtask-serde-allow: hand-built Value of owned Strings: serialization is infallible, and the .expect on the next line states that invariant; propagation would add an unreachable error arm
                .expect("serializing a hand-built Value cannot fail");
            durable_verb(
                "legal-hold release",
                |config| {
                    // The principal travels in the BODY, symmetric with
                    // `forget`: every principal in this tree is email-shaped
                    // and `@` is outside the door's path-segment charset, so a
                    // path segment would have cost either a wider charset for
                    // one route or percent-decoding inside the door.
                    door_client::exchange(
                        config,
                        "POST",
                        "/v1/legal-holds/release",
                        Some(("application/json", &body)),
                        DEFAULT_ROUTE_BUDGET,
                    )
                },
                || legal_hold_release_offline_arm(principal, color),
            )
        }
    }
}

/// `legal-hold list` — in-process durable reader (D-16-1-A): the holds read
/// READ-ONLY through `maos_audit`, never the write-capable adapter (Trap 9:
/// a write-capable open can hold the sqlite lock past the daemon's
/// `busy_timeout` and trip its panic-on-write-error) and no child spawn.
/// In tenant mode the holds live in the GLOBAL database while the resolved
/// path may be a team shard, so both candidates are consulted and merged.
fn legal_hold_list_reader() -> ExitCode {
    let resolved = default_transparency_log_path();
    let global = maos_audit::default_transparency_log_path();
    let mut paths = vec![resolved];
    if global != paths[0] {
        paths.push(global);
    }
    let mut holds: Vec<maos_audit::LegalHoldRow> = Vec::new();
    for path in &paths {
        match maos_audit::list_legal_holds_readonly(path) {
            Ok(rows) => holds.extend(rows),
            Err(error) => {
                eprintln!(
                    "maosctl: legal-hold list — failed to read {}: {error}",
                    path.display()
                );
                return ExitCode::from(1);
            }
        }
    }
    holds.sort_by(|a, b| a.principal_id.cmp(&b.principal_id));
    holds.dedup_by(|a, b| a.principal_id == b.principal_id);
    holds.sort_by_key(|hold| (hold.requested_at_ns, hold.principal_id.clone()));
    match serde_json::to_string(&holds) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("maosctl: legal-hold list — encode failed: {error}");
            ExitCode::from(1)
        }
    }
}

/// The offline legal-hold-release one-shot: env channel unchanged from
/// Story 13.5b; the child takes the exclusive store-lock set and prints its
/// own `running offline` line after acquiring.
fn legal_hold_release_offline_arm(principal: &str, color: ColorChoice) -> ExitCode {
    #[cfg(unix)]
    {
        let bin = maos_bin_path();
        let mut cmd = std::process::Command::new(&bin);
        cmd.env("MAOS_ONE_SHOT", "legal-hold-release");
        cmd.env("MAOS_LEGAL_HOLD_PRINCIPAL", principal);
        if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
            cmd.env("NO_COLOR", "1");
        }
        exec_and_forward(&mut cmd, &bin)
    }
    #[cfg(not(unix))]
    {
        let _ = (principal, color);
        door_client::offline_unsupported("legal-hold release")
    }
}

/// Story 9.2 (FR45) — `maosctl forget`: a durable verb — door when it
/// answers, the one-shot child otherwise. GDPR erasure (FR45/FR65) must
/// never require `maos init`: with no door configured the child runs
/// DIRECTLY (AC5 order 1).
fn dispatch_forget(args: &ForgetArgs, color: ColorChoice) -> ExitCode {
    let principal = args.principal.trim();
    if principal.is_empty() {
        eprintln!("maosctl: forget requires a non-empty --principal");
        return ExitCode::from(2);
    }
    let body = serde_json::to_vec(&serde_json::json!({
        "principal": principal,
        "reason": args.reason,
    }))
    .expect("serializing a hand-built Value cannot fail");
    durable_verb(
        "forget",
        |config| {
            door_client::exchange(
                config,
                "POST",
                "/v1/memory/forget",
                Some(("application/json", &body)),
                DEFAULT_ROUTE_BUDGET,
            )
        },
        || forget_offline_arm(principal, args.reason.as_deref(), color),
    )
}

/// The offline forget one-shot: env channel unchanged from Story 9.2; the
/// child takes the exclusive store-lock set (500 ms retry) and refuses with
/// 69 `StoreInUse` when an offline operation or daemon holds any of it.
fn forget_offline_arm(principal: &str, reason: Option<&str>, color: ColorChoice) -> ExitCode {
    #[cfg(unix)]
    {
        let bin = maos_bin_path();
        let mut cmd = std::process::Command::new(&bin);
        cmd.env("MAOS_ONE_SHOT", "forget");
        cmd.env("MAOS_FORGET_PRINCIPAL", principal);
        if let Some(reason) = reason {
            cmd.env("MAOS_FORGET_REASON", reason);
        }
        if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
            cmd.env("NO_COLOR", "1");
        }
        exec_and_forward(&mut cmd, &bin)
    }
    #[cfg(not(unix))]
    {
        let _ = (principal, reason, color);
        door_client::offline_unsupported("forget")
    }
}

/// `posture --shift` over the door: the daemon shifts the LIVE PolicyTable
/// of the named Spirit's pid (the one-shot shifted pid 0 in a fresh table —
/// measured harmless on a running daemon, D-16-1-A).
fn dispatch_posture(args: &PostureArgs, _color: ColorChoice) -> ExitCode {
    if let Some(code) = checked_id("posture", &args.spirit) {
        return code;
    }
    let posture = match args.shift {
        PostureChoice::Cautious => "cautious",
        PostureChoice::Assistive => "assistive",
        PostureChoice::AutonomousWithHalt => "autonomous-with-halt",
    };
    let body = serde_json::to_vec(&serde_json::json!({ "posture": posture })) // xtask-serde-allow: same: hand-built Value over a &'static str posture label
        .expect("serializing a hand-built Value cannot fail");
    door_verb("posture", |config| {
        door_client::exchange(
            config,
            "POST",
            &format!("/v1/spirits/{}/posture", args.spirit),
            Some(("application/json", &body)),
            DEFAULT_ROUTE_BUDGET,
        )
    })
}

fn dispatch_halt(args: &HaltArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        HaltOp::List { spirit, limit } => halt_list_reader(spirit.as_deref(), *limit),
        HaltOp::Resolve {
            halt_id,
            spirit,
            kind,
            text,
            operator_policy,
        } => {
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
            if let Some(code) = checked_id("halt resolve", halt_id) {
                return code;
            }
            if let Some(code) = checked_id("halt resolve", spirit) {
                return code;
            }
            // The daemon checks the HaltRegistry (an unknown or already-
            // resolved halt is its typed 404 `HaltNotPending`); the client
            // preflight is gone with the fabricated-halt one-shot.
            let resolution = match kind {
                ResolutionKindChoice::ProvidedContext => "provided_context",
                ResolutionKindChoice::AcceptedHalt => "accepted_halt",
                ResolutionKindChoice::AuthorizedOverride => "authorized_override",
            };
            // The rationale the door carries: the missing context for a
            // provided-context resolution, the authorizing policy reference
            // for an authorized override.
            let rationale = text.clone().or_else(|| operator_policy.clone());
            let body = serde_json::to_vec(&serde_json::json!({
                "spirit_id": spirit,
                "resolution": resolution,
                "rationale": rationale,
            }))
            .expect("serializing a hand-built Value cannot fail");
            door_verb("halt resolve", |config| {
                door_client::exchange(
                    config,
                    "POST",
                    &format!("/v1/halts/{halt_id}/resolve"),
                    Some(("application/json", &body)),
                    DEFAULT_ROUTE_BUDGET,
                )
            })
        }
    }
}

/// `halt list` — in-process durable reader (D-16-1-A): EpistemicHalt frames
/// read READ-ONLY through `maos_audit::query`, in the same
/// timestamp-ascending order and with the same limit semantics the old
/// child's `query_frames` used. No spawn, and never the write-capable
/// adapter (Trap 9). The spirit filter keeps the TL preflight — offline
/// readers have no daemon to resolve the name for them (D-16-1-K).
fn halt_list_reader(spirit: Option<&str>, limit: u32) -> ExitCode {
    let db_path = default_transparency_log_path();
    let mut filter = maos_audit::AuditFilter {
        kind: Some("epistemic.halt".to_owned()),
        limit: Some(limit as usize),
        ..Default::default()
    };
    if let Some(name) = spirit {
        match resolve_spirit_pid(name, &db_path, false) {
            Ok(pairs) => {
                if let Some((boot_nonce, pid)) = pairs.first() {
                    filter.boot_nonce = Some(*boot_nonce);
                    filter.spirit_pid = Some(*pid);
                }
            }
            Err(diag) => {
                eprintln!("maosctl: halt list — {diag}");
                return ExitCode::from(2);
            }
        }
    }
    let entries = match maos_audit::query(&db_path, filter) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("maosctl: halt list — query failed: {error}");
            return ExitCode::from(1);
        }
    };
    let mut unparseable = 0usize;
    for entry in &entries {
        let id_display: String = entry.frame_id_hex.chars().take(8).collect();
        let (record, halt_id) = classify_halt_record(&entry.payload);
        if record == "unparseable" {
            unparseable += 1;
        }
        let json_line = match serde_json::to_string(&serde_json::json!({
            "frame_id": id_display,
            "timestamp_ns": entry.timestamp_ns,
            "kind": entry.kind,
            "intent": entry.intent,
            "halt_id": halt_id,
            "record": record,
        })) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("maosctl: halt list — serialization error for frame: {e}");
                continue;
            }
        };
        println!("{json_line}");
    }
    eprintln!("maosctl: halt list — {} halts shown", entries.len());
    if unparseable > 0 {
        eprintln!("maosctl: halt list — {unparseable} halt row(s) had an unparseable payload");
    }
    ExitCode::SUCCESS
}

/// Story 16-2 / D-16-2-E — classify one `epistemic.halt` row's `record`,
/// in THIS order (the order is the correctness: a serialized `HaltReceipt`
/// parses as `EpistemicHaltPayload` too, whose only required field is
/// `halt_id` — raised-first classification would label every receipt
/// `raised`; probed against real `maos-domain` in validation round 3):
///
/// 1. empty payload                       ⇒ `termination_marker`, id `null`
///    (the row `terminate_spirit` writes before each receipt);
/// 2. parses as `HaltReceipt`             ⇒ `termination_receipt` with its
///    id — EXCEPT a `term-…` id, the kernel's synthetic no-halt form
///    (written when NOTHING was pending)  ⇒ `termination_no_pending`;
/// 3. parses as `EpistemicHaltPayload`    ⇒ `raised` with its id;
/// 4. anything else                       ⇒ `unparseable`, id `null`
///    (counted on stderr by the caller).
///
/// Nothing is filtered: a reader that hides rows lies about the registry
/// (§15 R6). Every `invoke_halt` caller mints ULIDs (§15 R9), which is what
/// makes the `term-` prefix sound.
pub fn classify_halt_record(payload: &str) -> (&'static str, Option<String>) {
    if payload.is_empty() {
        return ("termination_marker", None);
    }
    if let Ok(receipt) = serde_json::from_str::<maos_domain::halt::HaltReceipt>(payload) {
        let id = receipt.halt_id.as_str().to_string();
        if id.starts_with("term-") {
            return ("termination_no_pending", Some(id));
        }
        return ("termination_receipt", Some(id));
    }
    if let Ok(serde_json::Value::Object(object)) =
        serde_json::from_str::<serde_json::Value>(payload)
    {
        if object
            .get("error")
            .is_some_and(serde_json::Value::is_string)
        {
            if let Some(halt_id) = object.get("halt_id").and_then(serde_json::Value::as_str) {
                return ("termination_receipt", Some(halt_id.to_string()));
            }
        }
    }
    if let Ok(raised) = serde_json::from_str::<maos_domain::frame::EpistemicHaltPayload>(payload) {
        return ("raised", Some(raised.halt_id));
    }
    ("unparseable", None)
}

/// `pause`/`resume` over the door: the daemon performs the scheduler
/// transition FIRST and journals only on success (D-16-1-L) — the one-shot
/// wrote "Paused" rows while the Spirit kept running (§4, measured).
fn dispatch_pause(args: &PauseArgs, _color: ColorChoice) -> ExitCode {
    if let Some(code) = checked_id("pause", &args.spirit) {
        return code;
    }
    door_verb("pause", |config| {
        door_client::exchange(
            config,
            "POST",
            &format!("/v1/spirits/{}/pause", args.spirit),
            None,
            DEFAULT_ROUTE_BUDGET,
        )
    })
}

fn dispatch_resume(args: &ResumeArgs, _color: ColorChoice) -> ExitCode {
    if let Some(code) = checked_id("resume", &args.spirit) {
        return code;
    }
    door_verb("resume", |config| {
        door_client::exchange(
            config,
            "POST",
            &format!("/v1/spirits/{}/resume", args.spirit),
            None,
            DEFAULT_ROUTE_BUDGET,
        )
    })
}

fn dispatch_orchestrator(args: &OrchestratorArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        OrchestratorOp::Queue {
            spirit,
            instruction,
        } => {
            if instruction.trim().is_empty() {
                eprintln!("maosctl: orchestrator queue — instruction must be non-empty");
                return ExitCode::from(2);
            }
            if let Some(code) = checked_id("orchestrator queue", spirit) {
                return code;
            }
            let body = serde_json::to_vec(&serde_json::json!({ "text": instruction })) // xtask-serde-allow: same: hand-built Value over an owned instruction String
                .expect("serializing a hand-built Value cannot fail");
            door_verb("orchestrator queue", |config| {
                door_client::exchange(
                    config,
                    "POST",
                    &format!("/v1/orchestrator/{spirit}"),
                    Some(("application/json", &body)),
                    DEFAULT_ROUTE_BUDGET,
                )
            })
        }
        OrchestratorOp::Status { spirit } => {
            if let Some(code) = checked_id("orchestrator status", spirit) {
                return code;
            }
            orchestrator_status(spirit)
        }
    }
}

/// The occupancy read renders `pending/capacity` in plain text (AC4 names
/// "reports `1/32`"); the raw door JSON shape is the server's business.
fn orchestrator_status(spirit: &str) -> ExitCode {
    match door_client::resolve_door() {
        Ok(config) => {
            match door_client::exchange(
                &config,
                "GET",
                &format!("/v1/orchestrator/{spirit}"),
                None,
                DEFAULT_ROUTE_BUDGET,
            ) {
                Ok(response) if response.status == 200 => {
                    let body = response.body_json();
                    let pending = body
                        .get("pending")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let capacity = body
                        .get("capacity")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    println!(
                        "maosctl: orchestrator status {spirit}: {pending}/{capacity} pending instructions"
                    );
                    ExitCode::SUCCESS
                }
                Ok(response) => door_client::map_response("orchestrator status", response),
                Err(DoorError::ConnectRefused) => {
                    door_client::door_class_connect_refused("orchestrator status", config.endpoint)
                }
                Err(error) => door_client::map_error("orchestrator status", error),
            }
        }
        Err(error) => door_client::map_error("orchestrator status", error),
    }
}

fn dispatch_revoke_token(args: &RevokeTokenArgs, _color: ColorChoice) -> ExitCode {
    // Validate hex format BEFORE the round trip (Trap 18: lowercase only).
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
    door_verb("revoke-token", |config| {
        door_client::exchange(
            config,
            "POST",
            &format!("/v1/tokens/{}/revoke", args.token_id),
            None,
            DEFAULT_ROUTE_BUDGET,
        )
    })
}

fn dispatch_revocations(args: &RevocationsArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        RevocationsOp::Import { file } => {
            if !file.exists() {
                eprintln!(
                    "maosctl: revocations import — file not found: {}",
                    file.display()
                );
                return ExitCode::from(1);
            }
            let mut crl = Vec::with_capacity(MAX_DOOR_BODY_BYTES.min(8192) + 1);
            let read_result = std::fs::File::open(file).and_then(|opened| {
                opened
                    .take((MAX_DOOR_BODY_BYTES + 1) as u64)
                    .read_to_end(&mut crl)
            });
            if let Err(error) = read_result {
                eprintln!(
                    "maosctl: revocations import — cannot read {}: {error}",
                    file.display()
                );
                return ExitCode::from(1);
            }
            if crl.len() > MAX_DOOR_BODY_BYTES {
                eprintln!(
                    "maosctl: revocations import — {} exceeds the 64 KiB door limit",
                    file.display()
                );
                return ExitCode::from(1);
            }
            // D-16-1-X: the CRL's own BYTES travel as the body — a path
            // would resolve in the daemon's cwd, which is not the
            // operator's. The daemon holds the trust anchor captured at its
            // boot; the body is opaque to the route. (The `--force` re-apply
            // flag died with the one-shot: the daemon owns re-apply policy.)
            door_verb("revocations import", |config| {
                door_client::exchange(
                    config,
                    "POST",
                    "/v1/revocations",
                    Some(("application/octet-stream", &crl)),
                    LONG_ROUTE_BUDGET,
                )
            })
        }
        RevocationsOp::List => door_verb("revocations list", |config| {
            door_client::exchange(config, "GET", "/v1/revocations", None, DEFAULT_ROUTE_BUDGET)
        }),
    }
}

fn dispatch_spirit(args: &SpiritArgs, _color: ColorChoice) -> ExitCode {
    match &args.op {
        SpiritOp::HotSwapPrecheck {
            spirit,
            from,
            to,
            attestation,
            keyring,
        } => {
            // The daemon prechecks the LOADED control block, so the
            // predecessor version is its own fact — an operator-supplied
            // `--from` would be a claim about state the daemon can simply
            // read (D-16-1-W). Vetting artifacts travel with `upgrade` only.
            if from.is_some() {
                eprintln!(
                    "maosctl: spirit hot-swap-precheck — --from is not carried over the door: \
                     the daemon prechecks the LOADED control block"
                );
                return ExitCode::from(2);
            }
            if attestation.is_some() || keyring.is_some() {
                eprintln!(
                    "maosctl: spirit hot-swap-precheck — attestation/keyring travel with \
                     `spirit upgrade`, not the door precheck"
                );
                return ExitCode::from(2);
            }
            if let Some(code) = checked_id("spirit hot-swap-precheck", spirit) {
                return code;
            }
            let manifest = match canonical_manifest("spirit hot-swap-precheck", to) {
                Ok(path) => path,
                Err(code) => return code,
            };
            let body = serde_json::to_vec(&serde_json::json!({
                "target_manifest": manifest.to_string_lossy(),
            }))
            .expect("serializing a hand-built Value cannot fail");
            door_verb("spirit hot-swap-precheck", |config| {
                door_client::exchange(
                    config,
                    "POST",
                    &format!("/v1/spirits/{spirit}/hot-swap-precheck"),
                    Some(("application/json", &body)),
                    LONG_ROUTE_BUDGET,
                )
            })
        }
        SpiritOp::Upgrade {
            spirit,
            to,
            from,
            candidates,
            plan,
            attestation,
            keyring,
            policy,
        } => {
            // `--policy migrator` names the kernel's multi-hop executor, which
            // the door does not expose: D-16-1-W is hot-swap only, because the
            // cold-swap arm starts an unadmitted successor under a new pid.
            if policy == &UpgradePolicyArg::Migrator {
                eprintln!(
                    "maosctl: spirit upgrade — --policy migrator is not on the operator \
                     door surface (hot-swap only, D-16-1-W)"
                );
                return ExitCode::from(2);
            }
            // ⚠ `--plan` IS carried. The execution guard that reads a
            // persisted plan runs daemon-side either way, so refusing only the
            // CREATION half would leave a guard nothing could ever arm and a
            // shipped operator flag with no implementation.
            if !*plan && (from.is_some() || !candidates.is_empty()) {
                eprintln!("maosctl: spirit upgrade --from/--candidates require --plan");
                return ExitCode::from(2);
            }
            if let Some(code) = checked_id("spirit upgrade", spirit) {
                return code;
            }
            // Candidate manifests are canonicalised for the SAME reason `--to`
            // is: a relative path would resolve in the daemon's working
            // directory, not the operator's.
            let mut canonical_candidates = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                match canonical_manifest("spirit upgrade --candidates", candidate) {
                    Ok(path) => canonical_candidates.push(path.to_string_lossy().into_owned()),
                    Err(code) => return code,
                }
            }
            let manifest = match canonical_manifest("spirit upgrade", to) {
                Ok(path) => path,
                Err(code) => return code,
            };
            // D-16-1-W: all operator-selected files are canonicalized before
            // crossing into the daemon's different working directory.
            let attestation = match attestation {
                Some(path) => {
                    match canonical_operator_file("spirit upgrade", "attestation", path) {
                        Ok(path) => Some(path.to_string_lossy().into_owned()),
                        Err(code) => return code,
                    }
                }
                None => None,
            };
            let keyring = match keyring {
                Some(path) => match canonical_operator_file("spirit upgrade", "keyring", path) {
                    Ok(path) => Some(path.to_string_lossy().into_owned()),
                    Err(code) => return code,
                },
                None => None,
            };
            let body = serde_json::to_vec(&serde_json::json!({
                "target_manifest": manifest.to_string_lossy(),
                "policy": match policy {
                    UpgradePolicyArg::HotSwap => "hot-swap",
                    UpgradePolicyArg::ColdSwap => "cold-swap",
                    UpgradePolicyArg::Migrator => unreachable!("refused above"),
                },
                "attestation": attestation,
                "vetter_keyring": keyring,
                "from_version": from,
                "candidates": canonical_candidates,
                "create_plan": plan,
            }))
            .expect("serializing a hand-built Value cannot fail");
            door_verb("spirit upgrade", |config| {
                door_client::exchange(
                    config,
                    "POST",
                    &format!("/v1/spirits/{spirit}/upgrade"),
                    Some(("application/json", &body)),
                    LONG_ROUTE_BUDGET,
                )
            })
        }
        SpiritOp::Inspect { spirit, sandbox } => {
            if let Some(code) = checked_id("spirit inspect", spirit) {
                return code;
            }
            if *sandbox {
                // The preserved sandbox surface: HEAD report and exit codes
                // (4 unauthorized / 5 not found / 1 transport).
                match fetch_live_sandbox_report(spirit) {
                    Ok(report) => {
                        println!("{report}");
                        ExitCode::SUCCESS
                    }
                    Err(SandboxInspectHttpError::Unauthorized) => {
                        eprintln!(
                            "maosctl: sandbox inspect unauthorized (operator bearer rejected)"
                        );
                        ExitCode::from(4)
                    }
                    Err(SandboxInspectHttpError::NotFound) => {
                        eprintln!("maosctl: sandbox report not found for Spirit '{spirit}'");
                        ExitCode::from(5)
                    }
                    Err(error) => {
                        eprintln!("maosctl: sandbox inspect failed: {error}");
                        ExitCode::from(1)
                    }
                }
            } else {
                spirit_status_report(spirit)
            }
        }
    }
}

/// `spirit inspect <id>` WITHOUT `--sandbox`: the live control-block view
/// from `GET /v1/spirits/{id}` — lifecycle_state read from
/// `SpiritControlBlock::current_state` and posture from the PolicyTable the
/// daemon actually enforces (AC1), replacing the HEAD refusal line.
fn spirit_status_report(spirit: &str) -> ExitCode {
    match door_client::resolve_door() {
        Ok(config) => {
            match door_client::exchange(
                &config,
                "GET",
                &format!("/v1/spirits/{spirit}"),
                None,
                DEFAULT_ROUTE_BUDGET,
            ) {
                Ok(response) if response.status == 200 => {
                    let body = response.body_json();
                    let field = |name: &str| {
                        body.get(name)
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("?")
                            .to_owned()
                    };
                    let pid = body
                        .get("pid")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    println!("spirit_id: {}", field("spirit_id"));
                    println!("pid: {pid}");
                    println!("boot_nonce: {}", field("boot_nonce"));
                    println!("lifecycle_state: {}", field("lifecycle_state"));
                    println!("posture: {}", field("posture"));
                    println!("posture_ceiling: {}", field("posture_ceiling"));
                    ExitCode::SUCCESS
                }
                Ok(response) => door_client::map_response("spirit inspect", response),
                Err(DoorError::ConnectRefused) => {
                    door_client::door_class_connect_refused("spirit inspect", config.endpoint)
                }
                Err(error) => door_client::map_error("spirit inspect", error),
            }
        }
        Err(error) => door_client::map_error("spirit inspect", error),
    }
}

/// The `--sandbox` surface's own error shape: it keeps its HEAD exit codes
/// (4 unauthorized / 5 not found / 1 everything else), so it does NOT use the
/// D-16-1-P mapping the mutating verbs share. Discovery now runs through the
/// door client like every other verb (env pair → `control.json`).
#[derive(Debug, thiserror::Error)]
enum SandboxInspectHttpError {
    #[error("{0}")]
    Discovery(String),
    #[error("connection to the operator door failed: {0}")]
    Connect(String),
    #[error("operator HTTP I/O: {0}")]
    Io(String),
    #[error("operator returned HTTP {0}")]
    UnexpectedStatus(u16),
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
}

/// Fetch a sandbox report from the already-running daemon.  This intentionally
/// has no one-shot process fallback: a fresh process has no live SCB state.
fn fetch_live_sandbox_report(spirit_id: &str) -> Result<String, SandboxInspectHttpError> {
    if spirit_id.is_empty()
        || !spirit_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(SandboxInspectHttpError::Discovery(
            "spirit id must match [A-Za-z0-9._-]".to_owned(),
        ));
    }
    let config = door_client::resolve_door().map_err(sandbox_door_error)?;
    let response = door_client::exchange(
        &config,
        "GET",
        &format!("/v1/spirits/{spirit_id}/sandbox"),
        None,
        door_client::DEFAULT_ROUTE_BUDGET,
    )
    .map_err(|error| match error {
        DoorError::ConnectRefused => {
            SandboxInspectHttpError::Connect(format!("{} refused", config.endpoint))
        }
        error => sandbox_door_error(error),
    })?;
    match response.status {
        200 => String::from_utf8(response.body)
            .map_err(|_| SandboxInspectHttpError::Io("report is not UTF-8".to_owned())),
        401 => Err(SandboxInspectHttpError::Unauthorized),
        404 => Err(SandboxInspectHttpError::NotFound),
        other => Err(SandboxInspectHttpError::UnexpectedStatus(other)),
    }
}

/// Discovery/validation failures render through the door client's typed
/// messages; they keep the legacy exit-1 shell around them.
fn sandbox_door_error(error: DoorError) -> SandboxInspectHttpError {
    match error {
        DoorError::NotInitialized(message) | DoorError::Config(message) => {
            SandboxInspectHttpError::Discovery(message)
        }
        DoorError::Unresponsive(detail) => SandboxInspectHttpError::Connect(detail),
        DoorError::ConnectRefused => {
            SandboxInspectHttpError::Connect("refused during discovery".to_owned())
        }
    }
}

/// Resolve `maos` binary path.
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
            let sibling = parent.join("maos");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    // 3. Fallback to PATH
    PathBuf::from("maos")
}

/// Execute a prepared `maos-bin` command and forward its stdout/stderr.
///
/// `std::process::Command::status()` inherits the parent's pipes but does not
/// consume the child's output. When the parent is itself run under a harness
/// that captures stdout/stderr via `Command::output()`, the child can deadlock
/// once the inherited pipes fill. Capturing the child's output inside maosctl
/// and writing it back keeps both the child and the harness unblocked.
fn exec_and_forward(cmd: &mut std::process::Command, bin: &std::path::Path) -> ExitCode {
    match cmd.output() {
        Ok(out) => {
            let _ = std::io::stdout().write_all(&out.stdout);
            let _ = std::io::stderr().write_all(&out.stderr);
            if out.status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(out.status.code().unwrap_or(2) as u8)
            }
        }
        Err(e) => {
            eprintln!(
                "maosctl: failed to execute maos-bin at '{}': {e}",
                bin.display()
            );
            ExitCode::from(2)
        }
    }
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
            host,
        }) => audit_sealed_export(
            spirit.as_deref(),
            range.as_deref(),
            output,
            audit_key,
            host.as_deref(),
            color,
        ),
        Some(AuditQuery::Keygen { output }) => audit_keygen(output),
        Some(AuditQuery::RecordCapture {
            capture,
            spirit,
            boot,
        }) => audit_record_capture(capture, spirit.as_deref(), *boot),
        Some(AuditQuery::VerifyBundle {
            bundle,
            pubkey,
            seed,
        }) => audit_verify_bundle(bundle, pubkey.as_deref(), seed.as_ref()),
        Some(AuditQuery::ReconcileHosts {
            bundle_a,
            pubkey_a,
            seed_a,
            bundle_b,
            pubkey_b,
            seed_b,
            receipt_out,
            receipt_key,
        }) => audit_reconcile_hosts(
            HostHalfArgs {
                bundle: bundle_a,
                pubkey: pubkey_a.as_deref(),
                seed: seed_a.as_ref(),
            },
            HostHalfArgs {
                bundle: bundle_b,
                pubkey: pubkey_b.as_deref(),
                seed: seed_b.as_ref(),
            },
            receipt_out.as_ref(),
            receipt_key,
        ),
        Some(AuditQuery::ScanCredentials {
            spirit,
            boot,
            range,
        }) => audit_scan_credentials(spirit.as_deref(), *boot, range.as_deref()),
        Some(AuditQuery::SubjectAccess { principal, format }) => {
            audit_subject_access(principal, *format, color)
        }
        Some(AuditQuery::PostureDelta {
            range,
            spirit,
            format,
        }) => audit_posture_delta(range, spirit.as_deref(), *format, color),
        Some(AuditQuery::Export {
            spirit,
            range,
            output,
            audit_key,
            redaction_policy,
        }) => audit_trajectory_export(
            spirit.as_deref(),
            range.as_deref(),
            output,
            audit_key,
            redaction_policy,
            color,
        ),
        Some(AuditQuery::Replay { bundle, output }) => audit_replay(bundle, output),
        Some(AuditQuery::CostReconcile {
            month,
            format,
            pricing,
        }) => audit_cost_reconcile(month, pricing, *format),
    }
}

/// Resolve a Spirit name to one or more `(boot_nonce, spirit_pid)` pairs.
///
/// Delegates to [`maos_audit::resolve_spirit_name`], which scans the TL for
/// SpiritAdmitted (kind 19) and `lifecycle.load` (kind 7) frames — D-16-1-K,
/// since `maos run` writes only the latter. Per Decision E: keyed on
/// `(boot_nonce, spirit_pid)` to discriminate pid reuse across boots.
/// Default: latest boot by `timestamp_ns` (never `max(boot_nonce)` — the
/// nonce is random). `all_boots` unions all incarnations.
///
/// After Story 16-1 this is NOT a door-verb preflight (the daemon resolves
/// names; unknown ⇒ typed 404): the remaining callers are the audit read
/// paths and the offline `halt list` reader, which have no daemon.
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
            Err(_) => {
                eprintln!(
                    "maosctl: audit query — unknown spirit '{name}' — no admission or load row names it in this Transparency Log"
                );
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
    host: Option<&str>,
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
            Ok(pairs) => match pairs.as_slice() {
                [] => {}
                [pair] => {
                    filter.spirit_pid = Some(pair.1);
                    filter.boot_nonce = Some(pair.0);
                }
                _ => {
                    eprintln!(
                        "maosctl: audit sealed-export — spirit '{name}' resolves to multiple (boot_nonce, pid) pairs; use --all-boots or disambiguate"
                    );
                    return ExitCode::from(2);
                }
            },
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
    // Story 9.4b AC-5 — region-pin the export when MAOS_REGION_HOME is set so a
    // foreign-region verifier cannot validate it (None ⇒ byte-identical to pre-9.4b).
    //
    // `j1-crosshost-2c` AC1.1 — bind the resolved region to a local BEFORE the
    // match consumes it. `sign_bundle` welds its signing seed from this same
    // value, so the pubkey printed below MUST be derived from it too: printing
    // `derive_pubkey(&seed)` while signing with the region-welded seed produced a
    // bundle nobody could verify with the key it advertised — and `demo-j1`
    // scrapes that printed key straight into `verify-bundle`.
    let region_home = match resolve_region_home() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — invalid region config: {e}");
            return ExitCode::from(2);
        }
    };
    let unsigned = match &region_home {
        Some(r) => unsigned.with_region(r),
        None => unsigned,
    };
    // `j1-crosshost-2c` AC2.1 — stamp the host discriminator. Refuse a blank tag:
    // `Some("")` would alter the canonical bytes while discriminating nothing.
    let unsigned = match host {
        Some(h) if h.trim().is_empty() => {
            eprintln!("maosctl: audit sealed-export — --host must not be empty");
            return ExitCode::from(2);
        }
        Some(h) => unsigned.with_host(h.trim()),
        None => unsigned,
    };

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
    // The key that ACTUALLY signed — derived from the same region binding.
    let pubkey = match &region_home {
        Some(r) => maos_audit::sealed_export::derive_region_pubkey(&seed, r),
        None => maos_audit::sealed_export::derive_pubkey(&seed),
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
            // AC1.3 — stdout carries the bundle, stderr carries the key, exactly
            // as `--output` mode already does. Without this line a stdout-mode
            // export is an unverifiable artifact you can produce by accident.
            eprintln!(
                "maosctl: sealed export written to stdout ({} entries, pubkey {})",
                signed.entries.len(),
                hex::encode(pubkey),
            );
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

// ─── J1 Tier-2 — journal-capture (record a signed-run capture as an audit row) ───

/// Egress posture the capture MUST declare (enforced egress is Epic-14 v2.0
/// hardening; a capture claiming "enforced" states a control that does not
/// exist and is refused).
const CAPTURE_EGRESS_DECLARED_NOT_ENFORCED: &str = "declared-not-enforced";
/// Redaction result the capture MUST assert.
const CAPTURE_REDACTION_VERIFIED: &str = "verified";
/// FS-jail posture the capture MUST declare, verbatim (j1-crosshost-2a AC4.1/4.2).
///
/// The ratified claim, in three checkable clauses: **the FS jail is the ADAPTER's,
/// DECLARED by MAOS in a hashed manifest, ENFORCED by the adapter, not by MAOS.**
/// All three are true at HEAD — the signed T6 run really did pass
/// `codex exec --sandbox workspace-write`, supplied by MAOS through the manifest
/// and bound into `argv_prefix_hash`, and codex really did enforce it. What MAOS
/// does NOT do is enforce it itself: `spawn_and_bridge` is a bare `Command::new`
/// with no namespace, no rlimit and no process group, and the basename allowlist
/// that selects the adapter has no realpath, hash or signature check, so a shim
/// named `codex` resolves and is admitted.
///
/// Mirrors [`CAPTURE_EGRESS_DECLARED_NOT_ENFORCED`] exactly: a stated posture a
/// capture cannot OVERCLAIM. `"maos-enforced"` is the overclaim direction and is
/// refused, because a reader who believes MAOS enforced the jail would draw a
/// stronger conclusion from the signature than the evidence supports.
const CAPTURE_FS_JAIL_ADAPTER_ENFORCED: &str = "adapter-enforced-maos-declared";

// ─── j1-crosshost-2c AC5.5 — the two-host posture, same shape ──────────────
//
// Two human-performed steps hold this claim up, and a reader who is told "these
// two hosts authenticated" will not guess that "these two hosts were
// INTRODUCED". They are stated here as PROPERTIES OF THE SYSTEM — value-
// constrained exactly like the two above — rather than as questions about our
// diligence, which belong in the story's blocking conditions.

/// The mTLS trust anchor between the two hosts is established **out of band by a
/// human operator**, not by the protocol. `2b` shipped the boot-nonce gap as a
/// stated boundary, not a fix: in a release build the nonce is always random, and
/// the documented path is an operator reading host A's nonce from its own
/// `cohort:daemon-started` TL row and hand-transcribing it into host B's static
/// peer-pin config. There is no automated channel.
///
/// The overclaim direction is `"protocol-negotiated"`, and it is refused.
const CAPTURE_TRUST_ANCHOR_OUT_OF_BAND: &str = "out-of-band-human-operator";

/// Host B's audit signing key is **provisioned separately, by hand**. Without two
/// independent roots "two hosts" degrades to "two identities": the region→team
/// derivation template exists to make keys derivable from ONE base seed, so a
/// welded per-host key would let one seed holder sign both halves.
///
/// The overclaim direction is `"derived-from-shared-root"`, and it is refused.
const CAPTURE_HOST_B_KEY_HAND_PROVISIONED: &str = "hand-provisioned-separately";

/// The honest shapes a two-host run may claim. `2b`'s mechanism proof is two real
/// OS processes on one box, each with its own config, audit DB and mTLS identity —
/// architecturally two Hosts in MAOS's vocabulary, but a reader hears "two
/// machines", so the capture must say which it was. Free prose is refused.
const CAPTURE_TWO_HOST_SHAPES: &[&str] = &["two-processes-one-box", "two-machines"];

/// The capture-doc schema `maosctl audit record-capture` validates before
/// journaling. Fields mirror the runbook Phase-4 capture template. Extra fields
/// are permitted (the operator may add notes) and are preserved verbatim in the
/// journaled row; only the fields below are REQUIRED, and two are
/// value-constrained so a capture that overclaims a control cannot be signed.
#[derive(Debug, serde::Deserialize)]
struct CaptureDoc {
    /// Named human signer (the accountable operator).
    #[serde(default)]
    signer: String,
    /// Live-agent identity + version (e.g. "codex 0.x.y").
    #[serde(default)]
    live_agent_identity: String,
    /// Non-secret command metadata — argv WITH the key redacted.
    #[serde(default)]
    command_metadata: String,
    /// Host-grant disposition (exact-match grant admitted; a mismatch refuses).
    #[serde(default)]
    host_grant_disposition: String,
    /// Audit + digest Transparency Log refs the run/digest cited.
    #[serde(default)]
    audit_refs: Vec<String>,
    /// Egress posture — MUST equal `declared-not-enforced`.
    #[serde(default)]
    egress: String,
    /// The egress-enforcement follow-up ID (Epic-14 v2.0).
    #[serde(default)]
    egress_followup: String,
    /// FS-jail posture — MUST equal `adapter-enforced-maos-declared`.
    #[serde(default)]
    fs_jail: String,
    /// The MAOS-enforced-isolation follow-up ID.
    #[serde(default)]
    fs_jail_followup: String,
    /// Redaction result — MUST equal `verified`.
    #[serde(default)]
    redaction_result: String,
    /// Run outcome (e.g. "worker completed; no secret persisted").
    #[serde(default)]
    outcome: String,
    /// `j1-crosshost-2c` AC5.5 — the two-host shape actually run. EMPTY for a
    /// single-host capture (every pre-2c capture stays valid); when non-empty this
    /// capture claims a two-host run and the three fields below become REQUIRED
    /// and value-constrained.
    #[serde(default)]
    two_host_shape: String,
    /// How the mTLS trust anchor between the two hosts was established.
    #[serde(default)]
    two_host_trust_anchor: String,
    /// How host B's audit signing key was provisioned.
    #[serde(default)]
    two_host_host_b_audit_key: String,
    /// The stranger's verification: `tools/verify-audit-bundle/verify.py`'s output.
    /// Our own `verify-bundle` is a self-check and does not discharge this.
    #[serde(default)]
    two_host_stranger_verification: String,
}

impl CaptureDoc {
    /// Fail-closed field validation. Every required field must be non-empty; the
    /// three control-statement fields are value-constrained so a dishonest capture
    /// (egress "enforced", FS jail "maos-enforced", redaction not "verified")
    /// cannot be journaled and then
    /// signed. Returns the first violation as an operator-facing message.
    fn validate(&self) -> Result<(), String> {
        let required = [
            ("signer", self.signer.trim()),
            ("live_agent_identity", self.live_agent_identity.trim()),
            ("command_metadata", self.command_metadata.trim()),
            ("host_grant_disposition", self.host_grant_disposition.trim()),
            ("egress_followup", self.egress_followup.trim()),
            ("fs_jail_followup", self.fs_jail_followup.trim()),
            ("outcome", self.outcome.trim()),
        ];
        for (name, val) in required {
            if val.is_empty() {
                return Err(format!(
                    "capture field `{name}` is required and must be non-empty"
                ));
            }
        }
        if self.audit_refs.iter().all(|r| r.trim().is_empty()) {
            return Err(
                "capture field `audit_refs` must list at least one Transparency Log ref \
                 (the audit/digest refs the run produced)"
                    .to_string(),
            );
        }
        if self.egress.trim() != CAPTURE_EGRESS_DECLARED_NOT_ENFORCED {
            return Err(format!(
                "capture field `egress` must be exactly \"{CAPTURE_EGRESS_DECLARED_NOT_ENFORCED}\" \
                 (this run DECLARES egress, it does not enforce it — enforced egress is Epic-14 \
                 v2.0 hardening); got \"{}\"",
                self.egress.trim()
            ));
        }
        if self.fs_jail.trim() != CAPTURE_FS_JAIL_ADAPTER_ENFORCED {
            return Err(format!(
                "capture field `fs_jail` must be exactly \
                 \"{CAPTURE_FS_JAIL_ADAPTER_ENFORCED}\" — the FS jail is the ADAPTER's, \
                 DECLARED by MAOS in a hashed manifest and ENFORCED by the adapter, NOT by \
                 MAOS (the spawn has no namespace, no rlimit and no process group, and the \
                 adapter is selected by basename with no realpath/hash/signature check); got \
                 \"{}\"",
                self.fs_jail.trim()
            ));
        }
        if self.redaction_result.trim() != CAPTURE_REDACTION_VERIFIED {
            return Err(format!(
                "capture field `redaction_result` must be exactly \"{CAPTURE_REDACTION_VERIFIED}\" \
                 (the injected key's value must be proven absent from the TL); got \"{}\"",
                self.redaction_result.trim()
            ));
        }
        self.validate_two_host()?;
        Ok(())
    }

    /// `j1-crosshost-2c` AC5.5 — a two-host capture may not overclaim.
    ///
    /// Skipped entirely when `two_host_shape` is empty, so every single-host
    /// capture written before this story stays valid. Once a capture DOES claim a
    /// two-host run, all four fields are required and three are value-constrained,
    /// exactly like `egress` and `fs_jail`: the overclaim direction is refused
    /// rather than documented, because a reader who believes the protocol
    /// negotiated the trust anchor, or that both keys descend from one root, would
    /// draw a stronger conclusion from the signature than the evidence supports.
    fn validate_two_host(&self) -> Result<(), String> {
        let shape = self.two_host_shape.trim();
        if shape.is_empty() {
            return Ok(());
        }
        if !CAPTURE_TWO_HOST_SHAPES.contains(&shape) {
            return Err(format!(
                "capture field `two_host_shape` must be one of {CAPTURE_TWO_HOST_SHAPES:?} — a \
                 reader hears \"two machines\", so the capture must say which it was; got \"{shape}\""
            ));
        }
        let anchor = self.two_host_trust_anchor.trim();
        if anchor != CAPTURE_TRUST_ANCHOR_OUT_OF_BAND {
            return Err(format!(
                "capture field `two_host_trust_anchor` must be exactly \
                 \"{CAPTURE_TRUST_ANCHOR_OUT_OF_BAND}\" — the boot-nonce pairing between these \
                 two hosts is performed BY A HUMAN with no automated channel, so a capture may \
                 not claim the protocol negotiated it; got \"{anchor}\""
            ));
        }
        let key = self.two_host_host_b_audit_key.trim();
        if key != CAPTURE_HOST_B_KEY_HAND_PROVISIONED {
            return Err(format!(
                "capture field `two_host_host_b_audit_key` must be exactly \
                 \"{CAPTURE_HOST_B_KEY_HAND_PROVISIONED}\" — without two INDEPENDENT roots \
                 \"two hosts\" degrades to \"two identities\", because one seed holder can \
                 derive every welded key; got \"{key}\""
            ));
        }
        if self.two_host_stranger_verification.trim().is_empty() {
            return Err(
                "capture field `two_host_stranger_verification` is required for a two-host \
                 claim: record `tools/verify-audit-bundle/verify.py`'s output. Verifying our \
                 own artifact with our own verify-bundle is a self-check, and the premise of \
                 this artifact is a claim a STRANGER can check"
                    .to_string(),
            );
        }
        Ok(())
    }
}

/// Parse, validate, canonicalize, and secret-screen a capture doc — the PURE
/// core of `record-capture` (no IO, no env), so the hard logic is unit-testable.
///
/// Returns the canonical (compact, key-ordered) capture bytes to journal, or an
/// operator-facing error. The SAME redaction filter used at every TL write is
/// applied here as a TRIPWIRE: a capture is non-secret by construction, so a
/// secret-shaped value is refused (never scrubbed-and-journaled, never echoed).
fn parse_and_validate_capture(raw: &str) -> Result<Vec<u8>, String> {
    let doc: CaptureDoc =
        serde_json::from_str(raw).map_err(|e| format!("capture is not valid JSON: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| format!("capture is not valid JSON: {e}"))?;

    doc.validate()?;

    let canonical =
        serde_json::to_vec(&value).map_err(|e| format!("canonicalization error: {e}"))?;

    // Credential tripwire: the SAME prefix rules as the TL redaction filter, but
    // NOT the hex-token heuristic — a capture legitimately cites 32-hex frame /
    // digest refs, yet must carry no API key / private key / cloud credential. A
    // credential present is an operator error: REFUSE (never echo it, never
    // journal it), naming only the class.
    if let Some(class) = maos_iac::adapter::redaction::detect_credential(&canonical) {
        return Err(format!(
            "REFUSED: the capture contains a secret-shaped value (class: {class}). A capture must \
             carry NON-SECRET metadata only (redact the key from argv before capturing). \
             Nothing was journaled."
        ));
    }
    Ok(canonical)
}

/// Write a validated, secret-free capture as a `run.capture` (kind 31) audit row.
///
/// Raw INSERT with a literal kind int + `HumanAuthored` origin, exactly like
/// `identity.asserted`=30 — a bin/CLI-boundary audit kind with ZERO kernel delta
/// (no `FrameKind` variant). Opened READ_WRITE (NOT CREATE): the kernel owns the
/// schema, so an absent TL fails closed. Returns the 16-byte frame_id.
fn journal_run_capture_row(
    db_path: &std::path::Path,
    spirit_pid: u32,
    boot_nonce: u64,
    payload: &[u8],
) -> Result<[u8; 16], String> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        format!(
            "no Transparency Log at {} ({e}). Run the spirit first to seed it.",
            db_path.display()
        )
    })?;

    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    // frame_id: 16 random bytes via SQLite (no extra RNG dep in maos-cli).
    let frame_vec: Vec<u8> = conn
        .query_row("SELECT randomblob(16)", [], |r| r.get(0))
        .map_err(|e| format!("frame-id generation failed: {e}"))?;

    conn.execute(
        "INSERT INTO transparency_log \
         (frame_id, timestamp_ns, spirit_pid, boot_nonce, capability_token, \
          kind, intent, payload_redacted, origin) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            &frame_vec[..],
            now_ns as i64,
            spirit_pid as i64,
            boot_nonce as i64,
            Option::<&[u8]>::None, // capability_token = NULL
            31i64,                 // kind — run.capture
            "run.capture",         // intent
            payload,               // validated, secret-free
            maos_domain::invariants::i3::FrameOrigin::HumanAuthored as i64, // origin = 0 (non-kernel)
        ],
    )
    .map_err(|e| format!("journal write failed: {e}"))?;

    let mut frame_id = [0u8; 16];
    if frame_vec.len() != 16 {
        return Err(format!(
            "frame-id generation returned {} bytes, expected 16",
            frame_vec.len()
        ));
    }
    frame_id.copy_from_slice(&frame_vec);
    Ok(frame_id)
}

/// J1 Tier-2 — journal a signed-run capture doc as a `run.capture` audit row so
/// a subsequent `sealed-export` signature covers it.
///
/// `sealed-export` signs the covered-window audit ROWS (not an arbitrary file
/// set), so a capture is only under the signature once it is in the TL.
fn audit_record_capture(
    capture: &std::path::Path,
    spirit: Option<&str>,
    boot: Option<u64>,
) -> ExitCode {
    // 1-4. Read, validate, canonicalize, secret-screen (pure core).
    let raw = match std::fs::read_to_string(capture) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "maosctl: audit record-capture — cannot read {}: {e}",
                capture.display()
            );
            return ExitCode::from(2);
        }
    };
    let canonical = match parse_and_validate_capture(&raw) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("maosctl: audit record-capture — {e}");
            return ExitCode::from(2);
        }
    };

    // 5. Resolve the spirit stamp so `sealed-export --spirit <same>` covers the
    // row. Omitting --spirit stamps a host-level attestation (pid/boot = 0),
    // covered by a `sealed-export --range <window>` export instead.
    let db_path = default_transparency_log_path();
    let (spirit_pid, boot_nonce): (u32, u64) = match spirit {
        Some(name) => {
            let all_boots = boot.is_some();
            match resolve_spirit_pid(name, &db_path, all_boots) {
                Ok(pairs) => {
                    let chosen = match boot {
                        Some(b) => pairs.iter().find(|(bn, _)| *bn == b).copied(),
                        None => pairs.first().copied(),
                    };
                    match chosen {
                        Some((bn, pid)) => (pid, bn),
                        None => {
                            eprintln!(
                                "maosctl: audit record-capture — spirit '{name}' has no matching boot {} in the TL",
                                boot.map(|b| b.to_string())
                                    .unwrap_or_else(|| "(latest)".to_string())
                            );
                            return ExitCode::from(2);
                        }
                    }
                }
                Err(diag) => {
                    eprintln!("maosctl: audit record-capture — {diag}");
                    return ExitCode::from(2);
                }
            }
        }
        None => (0, 0),
    };

    // 6. Journal the row.
    let frame_id = match journal_run_capture_row(&db_path, spirit_pid, boot_nonce, &canonical) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("maosctl: audit record-capture — {e}");
            return ExitCode::from(2);
        }
    };

    let frame_hex = hex::encode(frame_id);
    eprintln!("maosctl: audit record-capture — journaled run.capture {frame_hex}");
    match spirit {
        Some(name) => eprintln!(
            "  now sign the covered window: maosctl audit sealed-export --spirit {name} --audit-key <signer.key> --output <bundle.json>"
        ),
        None => eprintln!(
            "  host-level attestation (pid/boot = 0); cover it with: maosctl audit sealed-export --range <window> --audit-key <signer.key> --output <bundle.json>"
        ),
    }
    ExitCode::SUCCESS
}

/// Resolve the expected attester key for `bundle` from exactly one operator-supplied
/// source.
///
/// `seed_path` DERIVES the key from the bundle's **claimed** region — the rule a
/// third-party verifier must follow, mirroring `verify_replication_bundle`'s
/// derive-from-claimed-identity contract. `pubkey_arg` is taken as already the
/// signing key. Neither path ever reads `signature_block.attester_pubkey`
/// (R-RG1): a bundle may not nominate the key that checks it.
fn resolve_verify_key(
    bundle: &maos_audit::sealed_export::AuditBundle,
    pubkey_arg: Option<&str>,
    seed_path: Option<&PathBuf>,
) -> Result<[u8; 32], String> {
    if let Some(path) = seed_path {
        let seed = maos_domain::audit_key::load_audit_key_seed(&Some(path.clone()))
            .map_err(|e| format!("cannot load base seed: {e}"))?;
        return match &bundle.region {
            Some(tag) => {
                let region = maos_domain::region::Region::canonicalize(tag)
                    .map_err(|e| format!("bundle claims an invalid region '{tag}': {e}"))?;
                Ok(maos_audit::sealed_export::derive_region_pubkey(
                    &seed, &region,
                ))
            }
            None => Ok(maos_audit::sealed_export::derive_pubkey(&seed)),
        };
    }
    let pubkey_arg = pubkey_arg.ok_or("one of --pubkey or --seed is required")?;
    // Resolve public key: distinguish file path from hex string.
    // A hex pubkey is exactly 64 hex chars (32 bytes). A file path typically
    // has an extension or directory separator — use that as the discriminator.
    let path = std::path::Path::new(pubkey_arg);
    // Treat as file if it has a file extension (.hex, .pub, .txt, etc.)
    // or contains a directory separator — avoids treating hex strings as
    // file paths on systems where a 64-char hex string happens to name a
    // real filesystem entry.
    let looks_like_file = path.extension().is_some()
        || pubkey_arg.contains('/')
        || pubkey_arg.contains(std::path::MAIN_SEPARATOR);
    let pubkey_hex = if looks_like_file && path.exists() {
        std::fs::read_to_string(pubkey_arg)
            .map_err(|e| format!("cannot read pubkey file '{pubkey_arg}': {e}"))?
            .trim()
            .to_string()
    } else if looks_like_file {
        return Err(format!("pubkey file '{pubkey_arg}' not found"));
    } else {
        pubkey_arg.to_string()
    };
    hex::decode(&pubkey_hex)
        .map_err(|e| format!("invalid pubkey hex: {e}"))?
        .try_into()
        .map_err(|bytes: Vec<u8>| {
            format!(
                "wrong pubkey length: expected 32 bytes (64 hex chars), got {} bytes ({} hex chars)",
                bytes.len(),
                pubkey_hex.len()
            )
        })
}

/// FR44 — verify a sealed-export bundle.
fn audit_verify_bundle(
    bundle: &PathBuf,
    pubkey_arg: Option<&str>,
    seed_path: Option<&PathBuf>,
) -> ExitCode {
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

    let pubkey_bytes = match resolve_verify_key(&bundle, pubkey_arg, seed_path) {
        Ok(k) => k,
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

/// One half of a two-host run as the operator supplied it on the command line.
struct HostHalfArgs<'a> {
    bundle: &'a PathBuf,
    pubkey: Option<&'a str>,
    seed: Option<&'a PathBuf>,
}

/// `j1-crosshost-2c` AC2.2 — reconcile two independently-signed halves of one
/// cross-host run on `frame_id`, and optionally emit a signed receipt.
///
/// Each half is verified against the key resolved for THAT half — derived from
/// its claimed region when `--seed-*` is used. `attester_pubkey` is never read to
/// decide anything (R-RG1), and two halves attested by one root are refused.
fn audit_reconcile_hosts(
    a: HostHalfArgs<'_>,
    b: HostHalfArgs<'_>,
    receipt_out: Option<&PathBuf>,
    receipt_key: &Option<PathBuf>,
) -> ExitCode {
    let (bundle_a, key_a, base_seed_a) = match load_host_half(&a) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("maosctl: audit reconcile-hosts — host A: {e}");
            return ExitCode::from(2);
        }
    };
    let (bundle_b, key_b, base_seed_b) = match load_host_half(&b) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("maosctl: audit reconcile-hosts — host B: {e}");
            return ExitCode::from(2);
        }
    };

    // §A6 review 2026-08-18 (P4): the in-code root check compares the two
    // RESOLVED keys, so ONE base seed under two claimed regions yields two
    // distinct keys and slips past `key_a == key_b`. When either half's key was
    // derived from a seed the operator supplied HERE, derive that seed under
    // the OTHER half's claimed identity and refuse any collision — that closes
    // both the same-seed-twice shape and the seed-plus-its-own-derived-pubkey
    // shape. (The two-invocation `MAOS_REGION_HOME` shape is undetectable at
    // reconcile and stays bounded by the sworn capture field; RELEASE-HOLDS
    // row 9 wording.)
    let one_root = |seed: &[u8; 32],
                    other_bundle: &maos_audit::sealed_export::AuditBundle,
                    other_key: &[u8; 32]| {
        let derived = match &other_bundle.region {
            Some(tag) => match maos_domain::region::Region::canonicalize(tag) {
                Ok(region) => maos_audit::sealed_export::derive_region_pubkey(seed, &region),
                Err(_) => maos_audit::sealed_export::derive_pubkey(seed),
            },
            None => maos_audit::sealed_export::derive_pubkey(seed),
        };
        derived == *other_key
    };
    if let Some(seed) = &base_seed_a {
        if seed == base_seed_b.as_ref().unwrap_or(seed) || one_root(seed, &bundle_b, &key_b) {
            eprintln!(
                "maosctl: audit reconcile-hosts — refused: both halves trace to ONE base \
                 seed; a two-host receipt requires independently provisioned roots (AC2.4)"
            );
            return ExitCode::from(1);
        }
    }
    if let Some(seed) = &base_seed_b {
        if one_root(seed, &bundle_a, &key_a) {
            eprintln!(
                "maosctl: audit reconcile-hosts — refused: both halves trace to ONE base \
                 seed; a two-host receipt requires independently provisioned roots (AC2.4)"
            );
            return ExitCode::from(1);
        }
    }

    let join = match maos_audit::sealed_export::reconcile_two_host_bundles(
        &bundle_a, &key_a, &bundle_b, &key_b,
    ) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("maosctl: audit reconcile-hosts — refused: {e}");
            return ExitCode::from(1);
        }
    };

    eprintln!(
        "maosctl: audit reconcile-hosts — OK (hosts {} + {}, {} shared frame_ids, {} A-only, {} B-only)",
        join.host_a,
        join.host_b,
        join.shared_frame_ids.len(),
        join.host_a_only.len(),
        join.host_b_only.len(),
    );
    eprintln!(
        "  claim scope: {}",
        maos_audit::sealed_export::TWO_HOST_CLAIM_SCOPE
    );

    let Some(path) = receipt_out else {
        return ExitCode::SUCCESS;
    };
    let operator_seed = match maos_domain::audit_key::load_audit_key_seed(receipt_key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl: audit reconcile-hosts — cannot load receipt key: {e}");
            return ExitCode::from(2);
        }
    };
    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let receipt = maos_audit::sealed_export::build_two_host_receipt(&operator_seed, &join, now_ns);
    let json = match serde_json::to_string_pretty(&receipt) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl: audit reconcile-hosts — serialization error: {e}");
            return ExitCode::from(2);
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("maosctl: audit reconcile-hosts — cannot create receipt dir: {e}");
            return ExitCode::from(2);
        }
    }
    if let Err(e) = std::fs::write(path, json) {
        eprintln!("maosctl: audit reconcile-hosts — receipt write error: {e}");
        return ExitCode::from(2);
    }
    eprintln!(
        "maosctl: two-host receipt written to {} (pubkey {})",
        path.display(),
        hex::encode(maos_audit::sealed_export::derive_pubkey(&operator_seed)),
    );
    ExitCode::SUCCESS
}

/// Read and parse one half plus the key it must verify against. When the key
/// was DERIVED from a seed path, the base seed travels back too — the one-root
/// closure in [`audit_reconcile_hosts`] needs it (§A6 review 2026-08-18).
fn load_host_half(
    half: &HostHalfArgs<'_>,
) -> Result<
    (
        maos_audit::sealed_export::AuditBundle,
        [u8; 32],
        Option<[u8; 32]>,
    ),
    String,
> {
    let text = std::fs::read_to_string(half.bundle)
        .map_err(|e| format!("read error on {}: {e}", half.bundle.display()))?;
    let bundle: maos_audit::sealed_export::AuditBundle =
        serde_json::from_str(&text).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    let base_seed = match half.seed {
        Some(path) => Some(
            maos_domain::audit_key::load_audit_key_seed(&Some(path.clone()))
                .map_err(|e| format!("cannot load base seed: {e}"))?,
        ),
        None => None,
    };
    let key = resolve_verify_key(&bundle, half.pubkey, half.seed)?;
    Ok((bundle, key, base_seed))
}

/// `j1-crosshost-2c` AC4.1 — the read-path credential scan.
///
/// Walks Transparency-Log rows that are ALREADY ON DISK and reports credential
/// shapes in the stored `payload_redacted` bytes. Every existing redaction call
/// site is pre-write, so nothing before this could see an escape after the fact.
///
/// Both classes are reported distinctly (ratified 2026-08-17): provider prefixes,
/// and the hex-run heuristic the write-path filter scrubs *silently*. A hit on
/// either is an escape — with one carve-out mirroring the write path: exact
/// 32-hex frame refs under `clause_sources` are retained BY CONTRACT (digest
/// rows), so they are not escapes (§A6 review 2026-08-18).
fn audit_scan_credentials(
    spirit: Option<&str>,
    boot: Option<u64>,
    range: Option<&str>,
) -> ExitCode {
    let db_path = default_transparency_log_path();
    let mut filter = maos_audit::AuditFilter::default();
    if let Some(name) = spirit {
        // §A6 review 2026-08-18 (P17): mirror record-capture's boot-scoped
        // resolution — the multi-pair arm used to tell the operator to
        // "disambiguate" with a flag this verb did not have.
        let all_boots = boot.is_some();
        match resolve_spirit_pid(name, &db_path, all_boots) {
            Ok(pairs) => {
                let chosen = match boot {
                    Some(b) => pairs.iter().find(|(bn, _)| *bn == b).copied(),
                    None => pairs.first().copied(),
                };
                match chosen {
                    Some((boot_nonce, spirit_pid)) => {
                        filter.spirit_pid = Some(spirit_pid);
                        filter.boot_nonce = Some(boot_nonce);
                    }
                    None => {
                        eprintln!(
                            "maosctl: audit scan-credentials — spirit '{name}' has no \
                             matching boot {} in the TL",
                            boot.map(|b| b.to_string())
                                .unwrap_or_else(|| "(latest)".to_string())
                        );
                        return ExitCode::from(2);
                    }
                }
            }
            Err(diag) => {
                eprintln!("maosctl: audit scan-credentials — {diag}");
                return ExitCode::from(2);
            }
        }
    }
    if let Some(range_str) = range {
        match parse_range(range_str) {
            Ok((since, until)) => {
                filter.since_ns = since;
                if let Some(u) = until {
                    filter.until_ns = Some(u);
                }
            }
            Err(e) => {
                eprintln!("maosctl: audit scan-credentials — {e}");
                return ExitCode::from(2);
            }
        }
    }

    let entries = match maos_audit::query(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit scan-credentials — no Transparency Log found at {}",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit scan-credentials — error: {e}");
            return ExitCode::from(2);
        }
    };

    let mut prefix_hits = 0usize;
    let mut hex_hits = 0usize;
    for entry in &entries {
        for shape in maos_iac::adapter::redaction::scan_stored_payload(entry.payload.as_bytes()) {
            if shape.is_prefix() {
                prefix_hits += 1;
            } else {
                hex_hits += 1;
            }
            // The finding names the row and the class. The offending bytes are
            // NEVER echoed: a scan that prints the secret it found is the leak.
            println!(
                "{{\"frame_id\":\"{}\",\"kind\":\"{}\",\"class\":\"{}\",\"escape\":true}}",
                entry.frame_id_hex,
                entry.kind,
                shape.class(),
            );
        }
    }

    eprintln!(
        "maosctl: audit scan-credentials — {} rows scanned, {} prefix escapes, {} hex-run escapes",
        entries.len(),
        prefix_hits,
        hex_hits,
    );
    if prefix_hits + hex_hits > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
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

// ─── FR46: Trajectory Export ────────────────────────────────────────────

/// FR46 — produce a signed trajectory export bundle.
fn audit_trajectory_export(
    spirit: Option<&str>,
    range: Option<&str>,
    output: &Option<PathBuf>,
    audit_key: &Option<PathBuf>,
    redaction_policy: &str,
    _color: ColorChoice,
) -> ExitCode {
    const VALID_REDUCTION_POLICIES: &[&str] = &["none", "all"];
    if !VALID_REDUCTION_POLICIES.contains(&redaction_policy) {
        eprintln!(
            "maosctl: audit export — unknown redaction policy '{}'. valid: {}",
            redaction_policy,
            VALID_REDUCTION_POLICIES.join(", ")
        );
        return ExitCode::from(2);
    }

    // Load audit signing key
    let seed = match maos_domain::audit_key::load_audit_key_seed(audit_key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl: audit export — {e}");
            return ExitCode::from(2);
        }
    };

    let db_path = default_transparency_log_path();
    let mut filter = maos_audit::AuditFilter::default();
    if let Some(name) = spirit {
        match resolve_spirit_pid(name, &db_path, false) {
            Ok(pairs) => match pairs.as_slice() {
                [] => {}
                [pair] => {
                    filter.spirit_pid = Some(pair.1);
                    filter.boot_nonce = Some(pair.0);
                }
                _ => {
                    eprintln!(
                        "maosctl: audit export — spirit '{name}' resolves to multiple (boot_nonce, pid) pairs; use --all-boots or disambiguate"
                    );
                    return ExitCode::from(2);
                }
            },
            Err(diag) => {
                eprintln!("maosctl: audit export — {diag}");
                return ExitCode::from(2);
            }
        }
    }

    if let Some(range_str) = range {
        match parse_range(range_str) {
            Ok((since, until)) => {
                filter.since_ns = since;
                if let Some(u) = until {
                    filter.until_ns = Some(u);
                }
            }
            Err(e) => {
                eprintln!("maosctl: audit export — {e}");
                return ExitCode::from(2);
            }
        }
    }

    // Use query_with_redaction to get redaction metadata
    let mut entries = match maos_audit::query_with_redaction(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit export — no Transparency Log found at {}. \
                 Run `maosctl run hello-spirit` first to seed the log.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit export — error: {e}");
            return ExitCode::from(2);
        }
    };

    // Apply redaction policy — fail-closed / default-deny
    let apply_redaction = redaction_policy != "none";
    let mut applied_redaction = false;
    if apply_redaction {
        for entry in &mut entries {
            if let Some(meta) = entry.redaction.as_ref() {
                // Redact: scrub intent to placeholder
                entry.intent = maos_audit::replay::render_placeholder(meta);
                applied_redaction = true;
            }
        }
    }

    // Build freshness
    let now_ns = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_nanos() as u64,
        Err(e) => {
            eprintln!("maosctl: audit export — system clock before Unix epoch: {e}");
            return ExitCode::from(2);
        }
    };
    let since_ns = entries.iter().map(|e| e.timestamp_ns).min().unwrap_or(0);
    let until_ns = entries
        .iter()
        .map(|e| e.timestamp_ns)
        .max()
        .unwrap_or(now_ns);

    let export_seq = match next_export_seq() {
        Ok(seq) => seq,
        Err(e) => {
            eprintln!("maosctl: audit export — {e}");
            return ExitCode::from(2);
        }
    };

    let freshness = maos_audit::sealed_export::FreshnessMetadata {
        export_timestamp_ns: now_ns,
        covered_window: maos_audit::sealed_export::CoveredWindow { since_ns, until_ns },
        export_seq,
    };

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

    // Build with trajectory schema version; redaction fields are part of the
    // signed payload so third-party verification covers them.
    let unsigned = maos_audit::sealed_export::build_trajectory_bundle(
        entries,
        i12_refs,
        i11_content,
        freshness,
        applied_redaction,
        redaction_policy.to_string(),
    );
    // Story 9.4b AC-5 — region-pin the trajectory export when MAOS_REGION_HOME is set.
    // `j1-crosshost-2c` AC1.2 — same defect, second site: bind the region local so
    // the printed pubkey can be derived from the identity that signed.
    let region_home = match resolve_region_home() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("maosctl: audit export — invalid region config: {e}");
            return ExitCode::from(2);
        }
    };
    let unsigned = match &region_home {
        Some(r) => unsigned.with_region(r),
        None => unsigned,
    };

    let signed = match maos_audit::sealed_export::sign_bundle(unsigned, &seed) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit export — signing error: {e}");
            return ExitCode::from(2);
        }
    };

    let json_bytes = match serde_json::to_string_pretty(&signed) {
        Ok(s) => s.into_bytes(),
        Err(e) => {
            eprintln!("maosctl: audit export — serialization error: {e}");
            return ExitCode::from(2);
        }
    };
    // The key that ACTUALLY signed — derived from the same region binding.
    let pubkey = match &region_home {
        Some(r) => maos_audit::sealed_export::derive_region_pubkey(&seed, r),
        None => maos_audit::sealed_export::derive_pubkey(&seed),
    };

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("maosctl: audit export — cannot create output dir: {e}");
                    return ExitCode::from(2);
                }
            }
            if let Err(e) = std::fs::write(path, &json_bytes) {
                eprintln!("maosctl: audit export — write error: {e}");
                return ExitCode::from(2);
            }
            eprintln!(
                "maosctl: trajectory export written to {} ({} entries, applied_redaction={}, pubkey {})",
                path.display(),
                signed.entries.len(),
                signed.applied_redaction,
                hex::encode(pubkey),
            );
        }
        None => {
            use std::io::Write;
            let stdout = std::io::stdout();
            if let Err(e) = stdout.lock().write_all(&json_bytes) {
                eprintln!("maosctl: audit export — write error: {e}");
                return ExitCode::from(2);
            }
            // AC1.3 — stdout carries the bundle, stderr carries the key.
            eprintln!(
                "maosctl: trajectory export written to stdout ({} entries, applied_redaction={}, pubkey {})",
                signed.entries.len(),
                signed.applied_redaction,
                hex::encode(pubkey),
            );
        }
    }

    ExitCode::SUCCESS
}

/// Return the next monotonic export sequence number.
///
/// Persists the last used value in a small JSON file next to the Transparency
/// Log so that repeated exports never reuse or regress the sequence, even if
/// the system clock jumps backwards.
fn next_export_seq() -> Result<u64, String> {
    let tl_path = default_transparency_log_path();
    let audit_dir = tl_path
        .parent()
        .ok_or("Transparency Log path has no parent directory")?;
    let state_path = audit_dir.join("export-seq.json");
    let last = match std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("last_export_seq").and_then(|v| v.as_u64()))
    {
        Some(seq) => seq,
        None => 0,
    };

    let now_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| format!("system clock before Unix epoch: {e}"))?
        .as_nanos() as u64;

    let seq = std::cmp::max(now_ns, last.saturating_add(1));

    let state = serde_json::json!({ "last_export_seq": seq });
    std::fs::create_dir_all(audit_dir)
        .map_err(|e| format!("cannot create audit state dir: {e}"))?;
    let state_json = serde_json::to_string(&state)
        .map_err(|e| format!("cannot serialize export-seq state: {e}"))?;
    std::fs::write(&state_path, &state_json)
        .map_err(|e| format!("cannot write export-seq state: {e}"))?;

    Ok(seq)
}

// ─── ADR-028: Replay ───────────────────────────────────────────────────

/// Maximum recursion depth when canonicalizing an untrusted bundle.
const REPLAY_SORT_VALUE_MAX_DEPTH: usize = 128;

/// ADR-028 — replay a sealed-export or trajectory bundle as a trace-shape doc.
fn audit_replay(bundle_path: &PathBuf, output: &Option<PathBuf>) -> ExitCode {
    // Read bundle file
    let bundle_bytes = match std::fs::read(bundle_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit replay — cannot read bundle: {e}");
            return ExitCode::from(2);
        }
    };

    let bundle_val: serde_json::Value = match serde_json::from_slice(&bundle_bytes) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("maosctl: audit replay — invalid JSON: {e}");
            return ExitCode::from(2);
        }
    };

    // Extract entries from the bundle
    let entries_val = match bundle_val.get("entries") {
        Some(v) => v,
        None => {
            eprintln!("maosctl: audit replay — bundle has no 'entries' field");
            return ExitCode::from(1);
        }
    };

    let entries: Vec<maos_audit::AuditEntry> = match serde_json::from_value(entries_val.clone()) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("maosctl: audit replay — cannot parse entries: {e}");
            return ExitCode::from(1);
        }
    };

    // Compute canonical bytes of the bundle (minus signature_block)
    // for the source_bundle_hash. Reuse the shared sealed_export canonicalizer
    // with a depth limit to avoid stack overflow on adversarial input.
    let mut bundle_for_hash = bundle_val.clone();
    if let Some(obj) = bundle_for_hash.as_object_mut() {
        obj.remove("signature_block");
    }
    let canonical_bytes =
        match sort_value_with_depth_limit(bundle_for_hash, REPLAY_SORT_VALUE_MAX_DEPTH) {
            Ok(sorted) => match serde_json::to_string(&sorted) {
                Ok(s) => s.into_bytes(),
                Err(e) => {
                    eprintln!("maosctl: audit replay — canonical serialize failed: {e}");
                    return ExitCode::from(2);
                }
            },
            Err(e) => {
                eprintln!("maosctl: audit replay — {e}");
                return ExitCode::from(2);
            }
        };

    // Run replay
    let trace_shape = match maos_audit::replay::replay(&entries, &canonical_bytes) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("maosctl: audit replay — {e}");
            return ExitCode::from(1);
        }
    };

    let shape_bytes = match maos_audit::replay::runner::replay_to_canonical_bytes(&trace_shape) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("maosctl: audit replay — {e}");
            return ExitCode::from(1);
        }
    };

    // Output pretty-printed for human readability
    let pretty = match serde_json::to_string_pretty(&trace_shape) {
        Ok(s) => s.into_bytes(),
        Err(e) => {
            eprintln!("maosctl: audit replay — serialization error: {e}");
            return ExitCode::from(1);
        }
    };

    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    eprintln!("maosctl: audit replay — cannot create output dir: {e}");
                    return ExitCode::from(2);
                }
            }
            if let Err(e) = std::fs::write(path, &pretty) {
                eprintln!("maosctl: audit replay — write error: {e}");
                return ExitCode::from(2);
            }
            eprintln!(
                "maosctl: trace-shape written to {} ({} frames, {} canonical bytes)",
                path.display(),
                trace_shape.frame_count,
                shape_bytes.len(),
            );
        }
        None => {
            use std::io::Write;
            let stdout = std::io::stdout();
            if let Err(e) = stdout.lock().write_all(&pretty) {
                eprintln!("maosctl: audit replay — write error: {e}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

/// Recursively sort JSON object keys with a depth limit.
///
/// Delegates to `maos_audit::sealed_export::sort_value` but guards against
/// adversarial deeply-nested input that could otherwise stack-overflow the CLI.
fn sort_value_with_depth_limit(
    value: serde_json::Value,
    max_depth: usize,
) -> Result<serde_json::Value, String> {
    fn recurse(
        v: serde_json::Value,
        depth: usize,
        max: usize,
    ) -> Result<serde_json::Value, String> {
        if depth > max {
            return Err("bundle nesting exceeds safe depth limit".to_string());
        }
        match v {
            serde_json::Value::Object(map) => {
                let mut sorted = serde_json::Map::new();
                for (k, v) in map.into_iter() {
                    sorted.insert(k, recurse(v, depth + 1, max)?);
                }
                Ok(serde_json::Value::Object(sorted))
            }
            serde_json::Value::Array(arr) => Ok(serde_json::Value::Array(
                arr.into_iter()
                    .map(|v| recurse(v, depth + 1, max))
                    .collect::<Result<_, _>>()?,
            )),
            other => Ok(other),
        }
    }
    Ok(maos_audit::sealed_export::sort_value(recurse(
        value, 0, max_depth,
    )?))
}

/// Resolve the tenancy-aware Transparency Log SQLite path.
///
/// Tenant routing is explicit: only an active collective tier plus a
/// non-empty canonical home team selects a shard. Untenanted commands retain
/// the historical global path.
fn default_transparency_log_path() -> PathBuf {
    let home_team = std::env::var("MAOS_LOOM_HOME_TEAM").ok();
    let collective_configured = std::env::var_os("MAOS_LOOM_POSTGRES").is_some();
    let path = maos_audit::transparency_log_path_for_tenant_mode(
        collective_configured,
        home_team.as_deref(),
    )
    .unwrap_or_else(|error| {
        eprintln!("maosctl: invalid tenant Transparency Log path: {error}");
        std::process::exit(2);
    });
    maos_audit::validate_transparency_log_path(&path).unwrap_or_else(|error| {
        eprintln!("maosctl: unsafe tenant Transparency Log path: {error}");
        std::process::exit(2);
    });
    if collective_configured {
        if let Some(team) = home_team
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let team = maos_domain::team::TeamId::new(team).unwrap_or_else(|error| {
                eprintln!("maosctl: invalid tenant Transparency Log team: {error}");
                std::process::exit(2);
            });
            maos_audit::validate_transparency_log_team_binding(&path, &team).unwrap_or_else(
                |error| {
                    eprintln!("maosctl: unbound tenant Transparency Log artifact: {error}");
                    std::process::exit(2);
                },
            );
        }
    }
    path
}

// ─── FR64: Cost Reconcile ──────────────────────────────────────────────

/// FR64 cost-reconcile report row.
#[derive(Debug, serde::Serialize)]
struct CostReportRow {
    principal: String,
    spirit_pid: u32,
    provider: String,
    model: String,
    tokens_in: i64,
    tokens_out: i64,
    cost_micro: u64,
}
/// FR64 cost-reconcile report.
#[derive(Debug, serde::Serialize)]
struct CostReport {
    month: String,
    rows: Vec<CostReportRow>,
    total_cost_micro: u64,
    attributed_cost_micro: u64,
    attributable_fraction: f64,
    /// Per-Spirit attributable fraction (SR-2).  Unlike the host-wide
    /// `attributable_fraction`, each entry only counts costs emitted by that
    /// spirit_pid.
    per_spirit_attributable_fraction: std::collections::BTreeMap<u32, f64>,
    warnings: Vec<String>,
}

/// Parse "YYYY-MM" into `(since_ns, until_ns)` nanosecond bounds.
///
/// Uses `chrono` for calendar math so negative/pre-epoch years and leap
/// months are handled consistently. Rejects months outside 1-12 and years
/// before 1970 to prevent silent wrap-around of the Unix-epoch offset.
fn parse_month_range(month: &str) -> Result<(u64, u64), String> {
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return Err("month must be YYYY-MM".into());
    }
    let year: i32 = parts[0]
        .parse()
        .map_err(|_| "invalid year in YYYY-MM".to_string())?;
    let mon: u32 = parts[1]
        .parse()
        .map_err(|_| "invalid month in YYYY-MM".to_string())?;
    if !(1..=12).contains(&mon) {
        return Err("month must be 1-12".into());
    }
    if year < 1970 {
        return Err("year must be >= 1970".into());
    }
    let since = chrono::NaiveDate::from_ymd_opt(year, mon, 1)
        .ok_or_else(|| "invalid YYYY-MM date".to_string())?
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let since_ns = since.and_utc().timestamp().max(0) as u64 * 1_000_000_000;

    let (next_year, next_mon) = if mon == 12 {
        (year + 1, 1)
    } else {
        (year, mon + 1)
    };
    let until = chrono::NaiveDate::from_ymd_opt(next_year, next_mon, 1)
        .ok_or_else(|| "invalid YYYY-MM date".to_string())?
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let until_ns = until.and_utc().timestamp().max(0) as u64 * 1_000_000_000;

    Ok((since_ns, until_ns))
}
///
/// R5: accumulates token counts per group, then multiplies by price once
/// using `u128` to avoid overflow, dividing by 1000 at the end.
fn build_cost_report(
    month: &str,
    entries: &[maos_audit::AuditEntry],
    pricing: &maos_domain::cost::ProviderPricingConfig,
) -> CostReport {
    use maos_domain::cost::{CostAttributionPayload, CostDimension, PrincipalRef};
    use std::collections::BTreeMap;

    // Group key: (principal_display, spirit_pid, provider, model)
    type Key = (String, u32, String, String);
    // Accumulator: (tokens_in, tokens_out)
    let mut groups: BTreeMap<Key, (i64, i64)> = BTreeMap::new();
    let mut warnings: Vec<String> = Vec::new();

    for entry in entries {
        let payload: CostAttributionPayload = match serde_json::from_str(&entry.payload) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(format!(
                    "skipping malformed cost payload for frame {}: {e}",
                    entry.frame_id_hex
                ));
                continue;
            }
        };

        let principal_display = match &payload.principal {
            PrincipalRef::Resolved { principal_id } => principal_id.clone(),
            PrincipalRef::Ambiguous { .. } | PrincipalRef::Unattributed => {
                "host-unallocated".to_string()
            }
        };

        let tokens_in = payload
            .dimensions
            .get(&CostDimension::TokensIn)
            .copied()
            .unwrap_or(0)
            .max(0);
        let tokens_out = payload
            .dimensions
            .get(&CostDimension::TokensOut)
            .copied()
            .unwrap_or(0)
            .max(0);

        let key = (
            principal_display,
            payload.spirit_pid,
            payload.provider.clone(),
            payload.model.clone(),
        );
        let acc = groups.entry(key).or_insert((0, 0));
        acc.0 = acc.0.saturating_add(tokens_in);
        acc.1 = acc.1.saturating_add(tokens_out);
    }

    // First pass: compute full-precision cost per group and the authoritative
    // total.  R5 requires exactly one division by 1000 at the window boundary.
    let mut row_precisions: Vec<(Key, (i64, i64), u128)> = Vec::with_capacity(groups.len());
    let mut total_cost_u128: u128 = 0;
    let mut attributed_cost_u128: u128 = 0;
    let mut per_spirit_cost: BTreeMap<u32, (u128, u128)> = BTreeMap::new();

    for (key, (tokens_in, tokens_out)) in &groups {
        let (input_price, output_price) = pricing
            .lookup(&key.2, &key.3)
            .map(|e| (e.input_price_micro_per_1k, e.output_price_micro_per_1k))
            .unwrap_or((0, 0));

        let cost_u128 = (*tokens_in as u128) * (input_price as u128)
            + (*tokens_out as u128) * (output_price as u128);

        total_cost_u128 += cost_u128;
        if key.0 != "host-unallocated" {
            attributed_cost_u128 += cost_u128;
        }
        let spirit_entry = per_spirit_cost.entry(key.1).or_insert((0, 0));
        spirit_entry.0 += cost_u128;
        if key.0 != "host-unallocated" {
            spirit_entry.1 += cost_u128;
        }
        row_precisions.push((key.clone(), (*tokens_in, *tokens_out), cost_u128));
    }

    let total_cost_micro = (total_cost_u128 / 1000) as u64;
    let attributed_cost_micro = (attributed_cost_u128 / 1000) as u64;

    let per_spirit_attributable_fraction: BTreeMap<u32, f64> = per_spirit_cost
        .iter()
        .map(|(pid, (total, attributed))| {
            let fraction = if *total == 0 {
                0.0
            } else {
                *attributed as f64 / *total as f64
            };
            (*pid, fraction)
        })
        .collect();

    // Second pass: assign each row a cost_micro that sums exactly to
    // total_cost_micro.  This preserves the single-division authority and
    // eliminates per-row rounding drift.
    let mut rows = Vec::with_capacity(row_precisions.len());
    let mut assigned_total: u64 = 0;
    for (i, (key, (tokens_in, tokens_out), cost_u128)) in row_precisions.iter().enumerate() {
        let cost_micro = if total_cost_u128 == 0 {
            0
        } else if i == row_precisions.len() - 1 {
            // Last row absorbs the residual so the column foots exactly.
            total_cost_micro - assigned_total
        } else {
            ((*cost_u128 * total_cost_micro as u128) / total_cost_u128) as u64
        };
        assigned_total += cost_micro;
        rows.push(CostReportRow {
            principal: key.0.clone(),
            spirit_pid: key.1,
            provider: key.2.clone(),
            model: key.3.clone(),
            tokens_in: *tokens_in,
            tokens_out: *tokens_out,
            cost_micro,
        });
    }

    let attributable_fraction = if total_cost_u128 == 0 {
        0.0
    } else {
        attributed_cost_u128 as f64 / total_cost_u128 as f64
    };

    CostReport {
        month: month.to_string(),
        rows,
        total_cost_micro,
        attributed_cost_micro,
        attributable_fraction,
        per_spirit_attributable_fraction,
        warnings,
    }
}

/// Format a cost report as human-readable plain text.
fn format_cost_report_plain(report: &CostReport) -> String {
    let mut out = String::new();
    use std::fmt::Write;
    let _ = writeln!(out, "Cost Reconcile Report — {}", report.month);
    let _ = writeln!(
        out,
        "  total={} µ$  attributed={} µ$  fraction={:.4}",
        report.total_cost_micro, report.attributed_cost_micro, report.attributable_fraction,
    );
    for row in &report.rows {
        let _ = writeln!(
            out,
            "  {:30} pid={:<5} {:12}/{:24} in={:<10} out={:<10} cost={} µ$",
            row.principal,
            row.spirit_pid,
            row.provider,
            row.model,
            row.tokens_in,
            row.tokens_out,
            row.cost_micro,
        );
    }
    if !report.per_spirit_attributable_fraction.is_empty() {
        let _ = writeln!(out, "\nPer-Spirit attributable fraction:");
        for (pid, frac) in &report.per_spirit_attributable_fraction {
            let _ = writeln!(out, "  pid={pid}: {frac:.4}");
        }
    }
    if !report.warnings.is_empty() {
        let _ = writeln!(out, "\nWarnings:");
        for w in &report.warnings {
            let _ = writeln!(out, "  ⚠ {w}");
        }
    }
    out
}
/// FR64 — cost-reconcile CLI entry point.
fn audit_cost_reconcile(month: &str, pricing_path: &str, format: AuditFormat) -> ExitCode {
    let db_path = default_transparency_log_path();

    // Parse month → time range.
    let (since_ns, until_ns) = match parse_month_range(month) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("maosctl: audit cost-reconcile — {e}");
            return ExitCode::from(2);
        }
    };

    // Load pricing config.
    let pricing_content = match std::fs::read_to_string(pricing_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("maosctl: audit cost-reconcile — cannot read pricing: {e}");
            return ExitCode::from(2);
        }
    };
    let pricing: maos_domain::cost::ProviderPricingConfig = match toml::from_str(&pricing_content) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("maosctl: audit cost-reconcile — invalid pricing config: {e}");
            return ExitCode::from(2);
        }
    };

    // Query cost-attribution frames for the month.
    let filter = maos_audit::AuditFilter {
        kind: Some("cost".to_string()),
        since_ns: Some(since_ns),
        until_ns: Some(until_ns),
        ..Default::default()
    };
    let entries = match maos_audit::query(&db_path, filter) {
        Ok(e) => e,
        Err(maos_audit::AuditError::Open(_)) => {
            eprintln!(
                "maosctl: audit cost-reconcile — no Transparency Log found at {}.",
                db_path.display()
            );
            return ExitCode::from(2);
        }
        Err(e) => {
            eprintln!("maosctl: audit cost-reconcile — error: {e}");
            return ExitCode::from(2);
        }
    };

    let report = build_cost_report(month, &entries, &pricing);

    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    match format {
        AuditFormat::Ndjson => {
            let json = match serde_json::to_string(&report) {
                Ok(j) => j,
                Err(e) => {
                    eprintln!("maosctl: audit cost-reconcile — encode error: {e}");
                    return ExitCode::from(2);
                }
            };
            if let Err(e) = writeln!(lock, "{json}") {
                eprintln!("maosctl: audit cost-reconcile — write error: {e}");
                return ExitCode::from(2);
            }
        }
        AuditFormat::Plain => {
            let text = format_cost_report_plain(&report);
            if let Err(e) = write!(lock, "{text}") {
                eprintln!("maosctl: audit cost-reconcile — write error: {e}");
                return ExitCode::from(2);
            }
        }
    }

    ExitCode::SUCCESS
}

/// Resolve the operator home region respecting `operator.toml` + `MAOS_REGION_HOME`
/// precedence.  Matches `RegionSection::resolve_from_env_and_disk()` semantics so
/// CLI sealed-exports region-pin identically to the in-process memory manager
/// (Story 9.4b split-brain fix).
fn resolve_region_home(
) -> Result<Option<maos_domain::region::Region>, maos_domain::region::RegionError> {
    let disk_tag = read_operator_toml_region_tag();
    maos_domain::region::Region::resolve_home(disk_tag.as_deref())
}

/// Read the `[region].home_region` value from `~/.config/maos/operator.toml`,
/// returning `None` when the file is absent, unparseable, or lacks the key.
fn read_operator_toml_region_tag() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let path = std::path::PathBuf::from(home)
        .join(".config")
        .join("maos")
        .join("operator.toml");
    let contents = std::fs::read_to_string(path).ok()?;
    let val: toml::Value = contents.parse().ok()?;
    val.get("region")?
        .get("home_region")?
        .as_str()
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accessibility::ColorChoice;
    use crate::cli::{Cli, ForgetArgs, InstallArgs, RunArgs, Subcommand};
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
            release_url: None,
            release_pubkey: None,
            verify_only: false,
            from_local: None,
            prefix: None,
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

    // ── J1 Tier-2: record-capture (journal-capture wiring) ───────────────

    /// A complete, non-secret capture doc. `audit_refs` are real 32-hex frame
    /// IDs — legitimately hex, and the tripwire must NOT flag them.
    fn valid_capture_json() -> String {
        serde_json::json!({
            "signer": "Myoungki Jung (Lunarpulse)",
            "live_agent_identity": "codex 0.5.0",
            "command_metadata": "codex exec --sandbox workspace-write <task>; CODEX_API_KEY inherited from the operator's environment — MAOS neither injects nor holds it (value redacted, scanned)",
            "host_grant_disposition": "exact-match grant admitted (codex @ OpenAI, T3)",
            "audit_refs": ["aabbccddeeff00112233445566778899", "99887766554433221100ffeeddccbbaa"],
            "egress": "declared-not-enforced",
            "egress_followup": "FOLLOWUP-EPIC14-V2.0-PACKET-EGRESS-ENFORCEMENT",
            "fs_jail": "adapter-enforced-maos-declared",
            "fs_jail_followup": "FOLLOWUP-EPIC14-MAOS-ENFORCED-WORKER-ISOLATION",
            "redaction_result": "verified",
            "outcome": "worker completed; no secret persisted",
            "note": "operator may add extra fields — preserved verbatim"
        })
        .to_string()
    }

    fn create_tl_schema(db_path: &std::path::Path) {
        let conn = rusqlite::Connection::open(db_path).unwrap();
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
    }

    #[test]
    fn record_capture_parses_flags() {
        use crate::cli::{AuditQuery, Subcommand};
        let cli = Cli::try_parse_from([
            "maosctl",
            "audit",
            "record-capture",
            "--capture",
            "/tmp/cap.json",
            "--spirit",
            "orchestrator",
            "--boot",
            "7",
        ])
        .unwrap();
        match &cli.command {
            Subcommand::Audit(args) => match &args.query {
                Some(AuditQuery::RecordCapture {
                    capture,
                    spirit,
                    boot,
                }) => {
                    assert_eq!(capture.to_str(), Some("/tmp/cap.json"));
                    assert_eq!(spirit.as_deref(), Some("orchestrator"));
                    assert_eq!(*boot, Some(7));
                }
                _ => panic!("expected RecordCapture subcommand"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }

    #[test]
    fn capture_validation_accepts_complete_nonsecret_doc() {
        // The happy path: a complete capture with hex TL refs round-trips to
        // canonical bytes that still contain every required field.
        let bytes = parse_and_validate_capture(&valid_capture_json())
            .expect("a complete non-secret capture must validate");
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["signer"], "Myoungki Jung (Lunarpulse)");
        assert_eq!(v["egress"], "declared-not-enforced");
        assert_eq!(v["redaction_result"], "verified");
        assert_eq!(
            v["note"],
            "operator may add extra fields — preserved verbatim"
        );
    }

    #[test]
    fn capture_validation_rejects_missing_signer() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["signer"] = serde_json::json!("   "); // whitespace only
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(err.contains("signer"), "must name the missing field: {err}");
    }

    #[test]
    fn capture_validation_refuses_egress_enforced_overclaim() {
        // The honesty control: a capture may not CLAIM enforced egress (which
        // does not exist until Epic-14). This must be refused, not journaled.
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["egress"] = serde_json::json!("enforced");
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(
            err.contains("egress") && err.contains("declared-not-enforced"),
            "must refuse the egress overclaim and state the required value: {err}"
        );
    }

    #[test]
    fn capture_validation_refuses_maos_enforced_fs_jail_overclaim() {
        // j1-crosshost-2a AC4.2 — the FS-jail claim mirrors the egress precedent
        // exactly: a stated posture that a capture cannot OVERCLAIM.
        //
        // The overclaim direction is what matters. MAOS genuinely DECLARES the jail
        // (codex's `--sandbox workspace-write` and claude's `--settings` document
        // both ride in the hashed `argv_prefix`), and the adapter genuinely ENFORCES
        // it. What MAOS does not do is enforce it: the spawn is a bare
        // `Command::new` with no namespace, no rlimit and no process group, and the
        // adapter is selected by BASENAME with no realpath, hash or signature check
        // — so a shim named `codex` resolves, is admitted, and satisfies the host
        // grant's `attested_image` by plain string equality. A capture claiming
        // MAOS enforced the jail would let a reader draw a stronger conclusion from
        // the signature than the evidence supports.
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["fs_jail"] = serde_json::json!("maos-enforced");
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(
            err.contains("fs_jail") && err.contains("adapter-enforced-maos-declared"),
            "must refuse the FS-jail overclaim and state the required value: {err}"
        );
        // The under-claim direction is refused too — the posture is a fixed
        // statement, not a free-text field, for the same reason `egress` is.
        v["fs_jail"] = serde_json::json!("none");
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(err.contains("fs_jail"), "the posture is exact-match: {err}");
    }

    /// `j1-crosshost-2c` AC5.5 — the two-host posture, same shape and same rule.
    ///
    /// A single-host capture is untouched: `two_host_shape` empty ⇒ the block is
    /// skipped, so every pre-2c capture stays valid. Once a capture CLAIMS a
    /// two-host run, the two human-performed steps become stated properties of the
    /// system, and the overclaim direction of each is refused.
    #[test]
    fn capture_validation_refuses_the_two_host_overclaim_directions() {
        // The existing single-host capture must still validate — additive, not a
        // new requirement on work already captured.
        parse_and_validate_capture(&valid_capture_json())
            .expect("a single-host capture must stay valid");

        let two_host = |mutate: &dyn Fn(&mut serde_json::Value)| -> String {
            let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
            v["two_host_shape"] = serde_json::json!("two-processes-one-box");
            v["two_host_trust_anchor"] = serde_json::json!("out-of-band-human-operator");
            v["two_host_host_b_audit_key"] = serde_json::json!("hand-provisioned-separately");
            v["two_host_stranger_verification"] =
                serde_json::json!("verify.py: signature OK (1 entries)");
            mutate(&mut v);
            v.to_string()
        };

        // The honest capture validates.
        parse_and_validate_capture(&two_host(&|_| {}))
            .expect("an honestly-bounded two-host capture must validate");

        // OVERCLAIM 1 — "the protocol negotiated the trust anchor". It did not: the
        // boot nonce is hand-transcribed by an operator and there is no automated
        // channel. A reader told "these two hosts authenticated" will not guess
        // that "these two hosts were INTRODUCED".
        let err = parse_and_validate_capture(&two_host(&|v| {
            v["two_host_trust_anchor"] = serde_json::json!("protocol-negotiated");
        }))
        .unwrap_err();
        assert!(
            err.contains("two_host_trust_anchor") && err.contains("BY A HUMAN"),
            "must refuse the trust-anchor overclaim and say why: {err}"
        );

        // OVERCLAIM 2 — "host B's key was derived from the shared root". That is the
        // exact property that collapses "two hosts" into "two identities": one seed
        // holder can derive every welded key, so one machine could sign both halves.
        let err = parse_and_validate_capture(&two_host(&|v| {
            v["two_host_host_b_audit_key"] = serde_json::json!("derived-from-shared-root");
        }))
        .unwrap_err();
        assert!(
            err.contains("two_host_host_b_audit_key") && err.contains("two identities"),
            "must refuse the shared-root key claim: {err}"
        );

        // OVERCLAIM 3 — free prose in the shape field. "two hosts" is exactly the
        // ambiguity the field exists to remove.
        for prose in ["two hosts", "two datacentres", "distributed"] {
            let err = parse_and_validate_capture(&two_host(&|v| {
                v["two_host_shape"] = serde_json::json!(prose);
            }))
            .unwrap_err();
            assert!(
                err.contains("two_host_shape"),
                "`{prose}` must be refused: the shape is exact-match, not prose: {err}"
            );
        }

        // And the STRANGER's path is not optional: our own verify-bundle is a
        // self-check, and no stranger has ever checked one of these artifacts.
        let err = parse_and_validate_capture(&two_host(&|v| {
            v["two_host_stranger_verification"] = serde_json::json!("  ");
        }))
        .unwrap_err();
        assert!(
            err.contains("STRANGER"),
            "a two-host claim must carry the field-agnostic verifier's output: {err}"
        );
    }

    #[test]
    fn capture_validation_requires_an_fs_jail_followup() {
        // The egress precedent pairs a stated gap with a NAMED follow-up so the
        // residual has an owner instead of being a sentence in a story file.
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["fs_jail_followup"] = serde_json::json!("  ");
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(
            err.contains("fs_jail_followup"),
            "a stated posture without a named follow-up is a gap with no owner: {err}"
        );
    }

    #[test]
    fn capture_validation_requires_verified_redaction() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["redaction_result"] = serde_json::json!("unverified");
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(
            err.contains("redaction_result") && err.contains("verified"),
            "must require a verified redaction result: {err}"
        );
    }

    #[test]
    fn capture_validation_requires_at_least_one_audit_ref() {
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        v["audit_refs"] = serde_json::json!([]);
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(
            err.contains("audit_refs"),
            "must require an audit ref: {err}"
        );
    }

    #[test]
    fn capture_tripwire_refuses_a_pasted_api_key() {
        // The core secret-hygiene control: a real key accidentally pasted into
        // the argv metadata is REFUSED and never journaled — and the error
        // names only the class, never the value.
        let mut v: serde_json::Value = serde_json::from_str(&valid_capture_json()).unwrap();
        let secret = "sk-proj-AAAABBBBCCCCDDDDEEEEFFFF0000";
        v["command_metadata"] = serde_json::json!(format!("codex exec; OPENAI_API_KEY={secret}"));
        let err = parse_and_validate_capture(&v.to_string()).unwrap_err();
        assert!(err.contains("REFUSED"), "must refuse a pasted key: {err}");
        assert!(
            !err.contains(secret),
            "the error must NOT echo the secret value: {err}"
        );
    }

    #[test]
    fn capture_tripwire_allows_hex_tl_refs() {
        // The false-positive guard: 32-hex frame refs must NOT be mistaken for a
        // capability token — the whole reason the tripwire is prefix-only.
        let bytes = parse_and_validate_capture(&valid_capture_json())
            .expect("hex TL refs must not trip the credential tripwire");
        let text = String::from_utf8(bytes).unwrap();
        assert!(
            text.contains("aabbccddeeff00112233445566778899"),
            "the hex refs must survive verbatim, not be redacted: {text}"
        );
    }

    #[test]
    fn record_capture_row_round_trips_through_audit_query() {
        // End-to-end at the DB seam: a journaled run.capture row is what
        // sealed-export's `maos_audit::query` reads back — same kind string,
        // same spirit stamp, payload intact, secret-free.
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("transparency.sqlite");
        create_tl_schema(&db_path);

        let canonical = parse_and_validate_capture(&valid_capture_json()).unwrap();
        let frame_id =
            journal_run_capture_row(&db_path, 4321, 9, &canonical).expect("row must write");

        // Query it back exactly as sealed-export does (filter by spirit stamp).
        let mut filter = maos_audit::AuditFilter::default();
        filter.spirit_pid = Some(4321);
        filter.boot_nonce = Some(9);
        let entries = maos_audit::query(&db_path, filter).unwrap();
        let cap = entries
            .iter()
            .find(|e| e.kind == "run.capture")
            .expect("the run.capture row must be queryable");

        assert_eq!(cap.frame_id_hex, hex::encode(frame_id));
        assert_eq!(cap.spirit_pid, 4321);
        assert_eq!(cap.boot_nonce, 9);
        assert!(cap.timestamp_ns > 0, "row must carry a real timestamp");
        assert!(
            cap.payload.contains("declared-not-enforced") && cap.payload.contains("Lunarpulse"),
            "the capture payload must be journaled intact: {}",
            cap.payload
        );
        assert!(
            !cap.payload.contains("sk-proj-") && !cap.payload.contains("sk-ant-"),
            "no secret may appear in the journaled capture: {}",
            cap.payload
        );
    }

    #[test]
    fn journal_run_capture_row_fails_closed_without_a_tl() {
        // No schema created: opening READ_WRITE on an absent TL must fail closed.
        let tmpdir = tempfile::TempDir::new().unwrap();
        let db_path = tmpdir.path().join("does-not-exist.sqlite");
        let canonical = parse_and_validate_capture(&valid_capture_json()).unwrap();
        let err = journal_run_capture_row(&db_path, 0, 0, &canonical).unwrap_err();
        assert!(
            err.contains("Transparency Log"),
            "absent TL must fail closed with a clear message: {err}"
        );
        assert!(
            !db_path.exists(),
            "the raw kind-31 writer must not create an absent team shard"
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
                Some(AuditQuery::VerifyBundle {
                    bundle,
                    pubkey,
                    seed,
                }) => {
                    assert_eq!(bundle.to_str(), Some("/tmp/bundle.json"));
                    assert_eq!(pubkey.as_deref(), Some("abc123"));
                    assert!(seed.is_none());
                }
                _ => panic!("expected AuditQuery::VerifyBundle"),
            },
            _ => panic!("expected Audit subcommand"),
        }
    }
    // ── FR45 forget dispatch parsing tests (Story 9.2) ───────────────

    #[test]
    fn forget_parses_principal_and_reason() {
        let cli = Cli::try_parse_from([
            "maosctl",
            "forget",
            "--principal",
            "urn:maos:principal:test",
            "--reason",
            "gdpr-art17",
        ])
        .expect("forget --principal --reason must parse");
        match &cli.command {
            Subcommand::Forget(args) => {
                assert_eq!(args.principal, "urn:maos:principal:test");
                assert_eq!(args.reason.as_deref(), Some("gdpr-art17"));
            }
            _ => panic!("expected Forget subcommand"),
        }
    }

    #[test]
    fn forget_reason_is_optional() {
        let cli = Cli::try_parse_from(["maosctl", "forget", "--principal", "p1"])
            .expect("forget with only --principal must parse");
        match &cli.command {
            Subcommand::Forget(args) => {
                assert_eq!(args.principal, "p1");
                assert!(args.reason.is_none());
            }
            _ => panic!("expected Forget subcommand"),
        }
    }

    #[test]
    fn dispatch_forget_rejects_empty_principal() {
        let args = ForgetArgs {
            principal: "".into(),
            reason: None,
        };
        let code = dispatch_forget(&args, ColorChoice::Auto);
        assert_ne!(code, ExitCode::SUCCESS);
    }
    #[test]
    fn cost_report_reads_payload_not_intent() {
        // Regression: build_cost_report must parse the JSON payload, not the
        // intent string. Real TL rows store "cost:inference-attribution" in
        // intent and the CostAttributionPayload JSON in payload_redacted.
        use maos_domain::cost::{
            AttributionConfidence, AttributionSource, CostAttributionPayload, CostDimension,
            PrincipalRef,
        };
        use std::collections::BTreeMap;

        let mut dims = BTreeMap::new();
        dims.insert(CostDimension::TokensIn, 1000);
        dims.insert(CostDimension::TokensOut, 500);
        let payload = CostAttributionPayload {
            schema_version: 1,
            timestamp_ns: 1_000_000,
            spirit_pid: 7,
            provider: "anthropic".into(),
            model: "claude-3".into(),
            principal: PrincipalRef::Resolved {
                principal_id: "user:alice".into(),
            },
            attribution_source: AttributionSource::WriteTargetProxy,
            attribution_confidence: AttributionConfidence::Exact,
            dimensions: dims,
        };
        let entry = maos_audit::AuditEntry {
            frame_id_hex: "aa".repeat(16),
            timestamp_ns: 1_000_000,
            spirit_pid: 7,
            boot_nonce: 1,
            capability_token_hex: None,
            kind: "cost.attribution".into(),
            intent: "cost:inference-attribution".into(),
            payload: serde_json::to_string(&payload).unwrap(),
            redaction: None,
        };
        let pricing = maos_domain::cost::ProviderPricingConfig::new(vec![
            maos_domain::cost::ProviderPricingEntry {
                provider: "anthropic".into(),
                model: "claude-3".into(),
                input_price_micro_per_1k: 3000,
                output_price_micro_per_1k: 15000,
            },
        ]);
        let report = build_cost_report("2026-06", &[entry], &pricing);
        assert_eq!(report.warnings.len(), 0, "must not warn on valid payload");
        assert_eq!(report.rows.len(), 1);
        assert_eq!(report.rows[0].principal, "user:alice");
        assert_eq!(report.rows[0].tokens_in, 1000);
        assert_eq!(report.rows[0].tokens_out, 500);
        // (1000*3000 + 500*15000) / 1000 = 10_500 µ$
        assert_eq!(report.total_cost_micro, 10_500);
    }

    // ── Story 9.4: install --from-local verification tests ──────────

    /// Dev seed matching `RELEASE_PUBKEY` (same as maos-audit tests).
    fn dev_seed() -> [u8; 32] {
        let hex_str = "794959d4c4dc813f968cd95eb4a45c4a02583a7c5211126e7b4583e4776d1c8d";
        let bytes: Vec<u8> = (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).unwrap())
            .collect();
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        seed
    }

    /// Create a staged release directory with SHA256SUMS, SHA256SUMS.sig, and a binary.
    fn staged_release_dir(
        binary_name: &str,
        binary_content: &[u8],
        seed: &[u8; 32],
    ) -> tempfile::TempDir {
        use maos_audit::release_verify::{generate_sha256sums, sha256_hex, sign_sha256sums};

        let dir = tempfile::tempdir().unwrap();
        let hash = sha256_hex(binary_content);
        let sums = generate_sha256sums(&[(binary_name.to_string(), hash)]);
        let sig = sign_sha256sums(sums.as_bytes(), seed);

        std::fs::write(dir.path().join(binary_name), binary_content).unwrap();
        std::fs::write(dir.path().join("SHA256SUMS"), sums.as_bytes()).unwrap();
        std::fs::write(dir.path().join("SHA256SUMS.sig"), &sig).unwrap();
        dir
    }
    #[test]
    fn install_verify_local_release_artifact() {
        let seed = dev_seed();
        let binary = b"maos v0.5.0 release binary stub";
        let binary_name = platform_binary_name().unwrap();
        let dir = staged_release_dir(binary_name, binary, &seed);

        let exit = install_from_local(
            dir.path().to_str().unwrap(),
            &maos_audit::release_verify::RELEASE_PUBKEY,
            true, // verify_only
            None,
        );
        assert_eq!(exit, ExitCode::SUCCESS);
    }

    #[test]
    fn install_verify_tampered_artifact_rejected() {
        let seed = dev_seed();
        let binary = b"maos v0.5.0 release binary stub";
        let binary_name = platform_binary_name().unwrap();
        let dir = staged_release_dir(binary_name, binary, &seed);

        // Tamper the binary after staging
        let bin_path = dir.path().join(binary_name);
        std::fs::write(&bin_path, b"tampered content").unwrap();

        let exit = install_from_local(
            dir.path().to_str().unwrap(),
            &maos_audit::release_verify::RELEASE_PUBKEY,
            true,
            None,
        );
        assert_eq!(exit, ExitCode::from(1));
    }

    #[test]
    fn install_verify_missing_sig_fails() {
        let dir = tempfile::tempdir().unwrap();
        let binary_name = platform_binary_name().unwrap();
        let binary = b"some binary";
        let hash = maos_audit::release_verify::sha256_hex(binary);
        let sums =
            maos_audit::release_verify::generate_sha256sums(&[(binary_name.to_string(), hash)]);

        std::fs::write(dir.path().join(binary_name), binary).unwrap();
        std::fs::write(dir.path().join("SHA256SUMS"), sums.as_bytes()).unwrap();
        // No SHA256SUMS.sig → fail-closed

        let exit = install_from_local(
            dir.path().to_str().unwrap(),
            &maos_audit::release_verify::RELEASE_PUBKEY,
            true,
            None,
        );
        assert_eq!(exit, ExitCode::from(2));
    }
}
