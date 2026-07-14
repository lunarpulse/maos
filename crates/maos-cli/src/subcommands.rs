//! v0.1-β subcommand dispatch. `audit query` is the first subcommand
//! with a real body (Story 1b.1). `run` and `install` land at 1b.5a.
//! All others remain stubs.

use std::io::Write;
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
    HaltArgs, HaltOp, ImportArgs, InstallArgs, MigrateArgs, MigrateOp, OrchestratorArgs,
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
        Subcommand::Forget(args) => dispatch_forget(args, color),
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
    }
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

/// Story 9.3b — `maosctl governance admit` shells to `maos-bin` via the
/// `MAOS_ONE_SHOT=governance-admit` env channel.  The kernel-side handler
/// writes the schema-lifecycle registry row + governance event frame.
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

/// v0.1-β lifecycle dispatch: shell out to `maos-bin` with
/// `MAOS_ONE_SHOT=<verb>` per Decision Register D2.
///
/// At v0.1-β each verb writes exactly one Lifecycle Journal entry and
/// exits. No supervisor, no mailbox, no `task.orphaned` emission — those
/// land in Epic 5 (Story 5.1) with a real supervised lifecycle. The
/// journal entry IS the observable v0.1 side-effect.
/// Decision Register D3: the shape is `spirit: Option<&str>` — all
/// three v0.1-β `*Args` structs are identical, but Epic 5 will
/// differentiate them (Stop will gain `--grace-period`, etc.), so the
/// distinct struct types in `cli.rs` are preserved.
///
/// At v0.1-β only the reference Spirit (`hello-spirit`) is valid; unknown
/// names are rejected here so the diagnostic is surfaced by the CLI.
/// When `MAOS_ACCESSIBILITY_SMOKE` is set the verb short-circuits to a
/// deterministic ASCII-only diagnostic for the accessibility cascade.
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

    // v0.1-β only admits the reference Spirit; reject unknown names here so the
    // diagnostic is surfaced by the CLI instead of the shell-out.
    if name != "hello-spirit" {
        eprintln!("maosctl: unknown spirit '{name}' — only 'hello-spirit' is available at v0.1-β");
        return ExitCode::from(2);
    }

    // Accessibility smoke path (Story 1b.5c): unit tests assert the ANSI-free
    // cascade without spawning the full composition root.
    if std::env::var_os("MAOS_ACCESSIBILITY_SMOKE").is_some() {
        eprintln!("maosctl: {verb} smoke ok for {name}");
        return ExitCode::SUCCESS;
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

    exec_and_forward(&mut cmd, &bin)
}

/// Story 9.2 (FR45) — `maosctl forget` shells to `maos-bin` via the
/// existing `MAOS_ONE_SHOT` env channel.  Principal and optional reason
/// are forwarded through `MAOS_FORGET_PRINCIPAL` / `MAOS_FORGET_REASON`.
fn dispatch_forget(args: &ForgetArgs, color: ColorChoice) -> ExitCode {
    let principal = args.principal.trim();
    if principal.is_empty() {
        eprintln!("maosctl: forget requires a non-empty --principal");
        return ExitCode::from(2);
    }

    let bin = maos_bin_path();
    let mut cmd = std::process::Command::new(&bin);
    cmd.env("MAOS_ONE_SHOT", "forget");
    cmd.env("MAOS_FORGET_PRINCIPAL", principal);
    if let Some(reason) = &args.reason {
        cmd.env("MAOS_FORGET_REASON", reason);
    }

    if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
        cmd.env("NO_COLOR", "1");
    }

    exec_and_forward(&mut cmd, &bin)
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

    exec_and_forward(&mut cmd, &bin)
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
            exec_and_forward(&mut cmd, &bin)
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
            exec_and_forward(&mut cmd, &bin)
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

    exec_and_forward(&mut cmd, &bin)
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

    exec_and_forward(&mut cmd, &bin)
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

            exec_and_forward(&mut cmd, &bin)
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

            exec_and_forward(&mut cmd, &bin)
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

    exec_and_forward(&mut cmd, &bin)
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

            exec_and_forward(&mut cmd, &bin)
        }
        RevocationsOp::List => {
            let bin = maos_bin_path();
            let mut cmd = std::process::Command::new(&bin);
            cmd.env("MAOS_ONE_SHOT", "revocations-list");

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            exec_and_forward(&mut cmd, &bin)
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

            exec_and_forward(&mut cmd, &bin)
        }
        SpiritOp::Upgrade {
            spirit,
            to,
            from,
            candidates,
            plan,
            policy,
        } => {
            if let Err(diag) = resolve_spirit_pid(spirit, &default_transparency_log_path(), false) {
                eprintln!("maosctl: spirit upgrade — {diag}");
                return ExitCode::from(1);
            }
            let manifest_path = std::path::Path::new(to);
            if !manifest_path.exists() {
                eprintln!("maosctl: spirit upgrade — manifest file not found: {to}");
                return ExitCode::from(1);
            }
            if *plan && (from.as_deref().is_none_or(str::is_empty) || candidates.is_empty()) {
                eprintln!(
                    "maosctl: spirit upgrade --plan requires non-empty --from and --candidates"
                );
                return ExitCode::from(2);
            }
            if !*plan && (from.is_some() || !candidates.is_empty()) {
                eprintln!("maosctl: spirit upgrade --from/--candidates require --plan");
                return ExitCode::from(2);
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
            if *plan {
                cmd.env("MAOS_UPGRADE_PLAN", "1");
                cmd.env(
                    "MAOS_UPGRADE_FROM_VERSION",
                    from.as_deref().expect("validated --plan --from"),
                );
                let candidates_json = match serde_json::to_string(candidates) {
                    Ok(json) => json,
                    Err(err) => {
                        eprintln!("maos: failed to serialize upgrade candidates: {err}");
                        return ExitCode::FAILURE;
                    }
                };
                cmd.env("MAOS_UPGRADE_CANDIDATES", candidates_json);
            }

            if std::env::var_os("NO_COLOR").is_some() || color == ColorChoice::Never {
                cmd.env("NO_COLOR", "1");
            }

            exec_and_forward(&mut cmd, &bin)
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

            exec_and_forward(&mut cmd, &bin)
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
            Err(_) => {
                eprintln!(
                    "maosctl: audit query — unknown spirit '{name}' — only 'hello-spirit' is available at v0.1-β"
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
    let unsigned = match resolve_region_home() {
        Ok(Some(r)) => unsigned.with_region(&r),
        Ok(None) => unsigned,
        Err(e) => {
            eprintln!("maosctl: audit sealed-export — invalid region config: {e}");
            return ExitCode::from(2);
        }
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
    let unsigned = match resolve_region_home() {
        Ok(Some(r)) => unsigned.with_region(&r),
        Ok(None) => unsigned,
        Err(e) => {
            eprintln!("maosctl: audit export — invalid region config: {e}");
            return ExitCode::from(2);
        }
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
            let pubkey = maos_audit::sealed_export::derive_pubkey(&seed);
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

/// Resolve the default Transparency Log SQLite path.
///
/// Delegates to [`maos_audit::default_transparency_log_path`] — the single
/// source of truth shared by `maos-bin` (write side) and `maos-cli` (read
/// side) to prevent path-drift data loss.
fn default_transparency_log_path() -> PathBuf {
    maos_audit::default_transparency_log_path()
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
