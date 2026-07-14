#![forbid(unsafe_code)]

//! Persisted migration-plan storage and execution guard.
//!
//! This composition-root module deliberately owns orchestration only: the
//! cohort crate resolves and hashes candidate chains, while every hop still
//! calls the kernel's existing single-hop `UpgradeOrchestrator::upgrade`.

use std::path::{Path, PathBuf};

use maos_cohort::{resolve_migration_chain, MigrationCandidate, MigrationPlan};
use maos_kernel_core::lifecycle::{UpgradeOrchestrator, UpgradePolicy, UpgradeReport};
use sha2::{Digest, Sha256};

pub fn create_plan(
    spirit_id: &str,
    from_version: &str,
    target_manifest: &Path,
    candidate_manifest_paths: &[String],
) -> Result<(PathBuf, MigrationPlan), String> {
    let mut paths = candidate_manifest_paths.to_vec();
    let target = target_manifest.display().to_string();
    if !paths.iter().any(|path| same_manifest_path(path, &target)) {
        paths.push(target);
    }

    let candidates = load_candidates(&paths)?;
    let to_version = MigrationCandidate::from_manifest_file(target_manifest)
        .map_err(|error| error.to_string())?
        .version;
    let chain = resolve_migration_chain(from_version, &to_version, &candidates)
        .map_err(|error| error.to_string())?;
    let plan = MigrationPlan::new(spirit_id, from_version, to_version, paths, chain);
    let path = required_plan_path(spirit_id)?;
    let bytes = serde_json::to_vec_pretty(&plan)
        .map_err(|error| format!("serialize migration plan: {error}"))?;
    maos_skill::store::atomic_write(&path, &bytes)
        .map_err(|error| format!("persist migration plan {}: {error}", path.display()))?;
    Ok((path, plan))
}

pub async fn upgrade_with_plan_guard(
    orchestrator: &UpgradeOrchestrator,
    spirit_id: &str,
    successor_manifest: &Path,
    policy: UpgradePolicy,
) -> Result<Vec<UpgradeReport>, String> {
    let Some((plan_path, plan)) = load_plan(spirit_id)? else {
        return orchestrator
            .upgrade(spirit_id, successor_manifest, policy)
            .await
            .map(|report| vec![report])
            .map_err(|error| error.to_string());
    };

    if plan.spirit_id != spirit_id {
        return Err(format!(
            "migration plan {} belongs to {}, not {}",
            plan_path.display(),
            plan.spirit_id,
            spirit_id
        ));
    }

    let requested_to = MigrationCandidate::from_manifest_file(successor_manifest)
        .map_err(|error| error.to_string())?
        .version;
    if requested_to != plan.to_version {
        return Err(format!(
            "migration plan {} targets {}, but this upgrade requests {}; re-plan with --plan \
             for the new target (fail-closed: refusing a silent target divergence)",
            plan_path.display(),
            plan.to_version,
            requested_to
        ));
    }

    let candidates = load_candidates(&plan.candidate_manifest_paths)?;
    let live_chain = resolve_migration_chain(&plan.from_version, &plan.to_version, &candidates)
        .map_err(|error| error.to_string())?;
    plan.verify_live_chain(&live_chain)
        .map_err(|error| error.to_string())?;

    let mut reports = Vec::with_capacity(live_chain.hops().len());
    for hop in live_chain.hops() {
        let manifest_path = plan
            .candidate_manifest_paths
            .iter()
            .zip(&candidates)
            .find_map(|(path, candidate)| (candidate == &hop.successor).then_some(path))
            .ok_or_else(|| {
                format!(
                    "approved migration hop {} -> {} has no candidate manifest path",
                    hop.predecessor_version, hop.successor.version
                )
            })?;
        let report = match orchestrator
            .upgrade(spirit_id, Path::new(manifest_path), policy)
            .await
        {
            Ok(report) => report,
            Err(error) => {
                // Mid-chain failure: the plan is a hash-committed one-shot approval,
                // not a retry journal. Clear it and require an explicit re-plan from
                // the spirit's current live version rather than silently re-running
                // already-completed hops from the original predecessor.
                if let Err(cleanup) = std::fs::remove_file(&plan_path) {
                    eprintln!(
                        "maos: warning: failed to remove aborted migration plan {}: {cleanup}",
                        plan_path.display()
                    );
                }
                return Err(format!(
                    "migration hop {} -> {} failed: {error}; the spirit remains at {}. \
                     The consumed plan was cleared — re-plan with --plan from {} to resume.",
                    hop.predecessor_version,
                    hop.successor.version,
                    hop.predecessor_version,
                    hop.predecessor_version
                ));
            }
        };
        reports.push(report);
    }

    if let Err(error) = std::fs::remove_file(&plan_path) {
        eprintln!(
            "maos: warning: failed to remove consumed migration plan {}: {error}",
            plan_path.display()
        );
    }
    Ok(reports)
}

fn load_plan(spirit_id: &str) -> Result<Option<(PathBuf, MigrationPlan)>, String> {
    let Some(path) = optional_plan_path(spirit_id) else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }

    let bytes = std::fs::read(&path)
        .map_err(|error| format!("read migration plan {}: {error}", path.display()))?;
    let plan: MigrationPlan = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse migration plan {}: {error}", path.display()))?;
    plan.validate_persisted_hash()
        .map_err(|error| error.to_string())?;
    Ok(Some((path, plan)))
}

fn load_candidates(paths: &[String]) -> Result<Vec<MigrationCandidate>, String> {
    paths
        .iter()
        .map(|path| {
            MigrationCandidate::from_manifest_file(Path::new(path))
                .map_err(|error| error.to_string())
        })
        .collect()
}

/// Treat two manifest path strings as the same on-disk file when they resolve
/// to the same canonical path, falling back to exact string equality. Prevents
/// a path-form mismatch (relative/absolute, `./` prefix, symlink) between `--to`
/// and `--candidates` from double-loading the target and tripping a spurious
/// `ForkAtSource`.
fn same_manifest_path(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

fn required_plan_path(spirit_id: &str) -> Result<PathBuf, String> {
    optional_plan_path(spirit_id).ok_or_else(|| {
        "MAOS_HOME is required to persist a migration --plan across maosctl invocations".into()
    })
}

fn optional_plan_path(spirit_id: &str) -> Option<PathBuf> {
    std::env::var_os("MAOS_HOME").map(|home| {
        let mut hasher = Sha256::new();
        hasher.update(spirit_id.as_bytes());
        let name = hex::encode(hasher.finalize());
        PathBuf::from(home)
            .join("upgrade-plans")
            .join(format!("{name}.json"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex};

    static MAOS_HOME_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct RestoreMaosHome(Option<std::ffi::OsString>);

    impl Drop for RestoreMaosHome {
        fn drop(&mut self) {
            match &self.0 {
                Some(value) => std::env::set_var("MAOS_HOME", value),
                None => std::env::remove_var("MAOS_HOME"),
            }
        }
    }

    fn write_candidate(path: &Path, version: &str, migrates_from: &[&str]) {
        let patterns = migrates_from
            .iter()
            .map(|pattern| format!("\"{pattern}\""))
            .collect::<Vec<_>>()
            .join(", ");
        std::fs::write(
            path,
            format!(
                "[class]\nname = \"marcus-agent\"\nversion = \"{version}\"\n\n[migrates_from]\nversions = [{patterns}]\n"
            ),
        )
        .expect("candidate manifest writes");
    }

    #[test]
    fn plan_is_atomically_persisted_under_maos_home_and_round_trips() {
        let _guard = MAOS_HOME_LOCK.lock().expect("MAOS_HOME lock");
        let temporary_home = tempfile::tempdir().expect("temporary MAOS_HOME");
        let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
        std::env::set_var("MAOS_HOME", temporary_home.path());

        let v2 = temporary_home.path().join("v2.toml");
        let v3 = temporary_home.path().join("v3.toml");
        write_candidate(&v2, "2.0", &["1.0"]);
        write_candidate(&v3, "3.0", &["2.0"]);
        let candidate_paths = vec![v2.display().to_string(), v3.display().to_string()];

        let (plan_path, plan) =
            create_plan("marcus-agent", "1.0", &v3, &candidate_paths).expect("plan persists");
        assert!(plan_path.is_file());
        assert_eq!(plan.schema_version, MigrationPlan::SCHEMA_VERSION);
        assert_eq!(plan.hops.len(), 2);

        let (_, loaded) = load_plan("marcus-agent")
            .expect("plan reads")
            .expect("plan exists");
        assert_eq!(loaded, plan);
        drop(restore);
    }

    // Story 12.5 §A7 plan-drift reflex (persisted-FILE path): the drift refusal
    // must come from a REAL hash comparison of the persisted plan FILE's
    // plan_hash vs the chain re-derived from the live candidate manifests on
    // disk — NOT an in-memory-only comparison. Persist a plan, then mutate a
    // candidate manifest on disk so the re-derived chain still resolves the same
    // path but hashes differently (a legal fan-in changes the hop's hashed
    // migrates_from set), and require the guard's re-derive-from-disk step to
    // refuse with EMigrationPlanDrift.
    #[test]
    fn persisted_plan_file_drift_is_refused_after_a_candidate_mutates_on_disk() {
        use maos_cohort::CohortError;

        let _guard = MAOS_HOME_LOCK.lock().expect("MAOS_HOME lock");
        let temporary_home = tempfile::tempdir().expect("temporary MAOS_HOME");
        let restore = RestoreMaosHome(std::env::var_os("MAOS_HOME"));
        std::env::set_var("MAOS_HOME", temporary_home.path());

        let v2 = temporary_home.path().join("v2.toml");
        let v3 = temporary_home.path().join("v3.toml");
        write_candidate(&v2, "2.0", &["1.0"]);
        write_candidate(&v3, "3.0", &["2.0"]);
        let candidate_paths = vec![v2.display().to_string(), v3.display().to_string()];

        let (_, plan) =
            create_plan("marcus-agent", "1.0", &v3, &candidate_paths).expect("plan persists");
        assert_eq!(plan.hops.len(), 2);

        // Drift the candidate set on disk: v2 now also accepts source 0.9 (a
        // legal fan-in — the 1.0 -> 2.0 -> 3.0 path still resolves), but the hop's
        // hashed migrates_from set has changed, so the re-derived chain hash no
        // longer matches the persisted plan.
        write_candidate(&v2, "2.0", &["1.0", "0.9"]);

        // Re-derive exactly as the guard does: read the persisted plan file, then
        // resolve over the live (mutated) candidates and compare hashes.
        let (_, reloaded) = load_plan("marcus-agent")
            .expect("plan file reads")
            .expect("plan file exists");
        reloaded
            .validate_persisted_hash()
            .expect("the plan FILE itself is intact — only the candidates drifted");
        let candidates =
            load_candidates(&reloaded.candidate_manifest_paths).expect("candidates re-read");
        let live_chain =
            resolve_migration_chain(&reloaded.from_version, &reloaded.to_version, &candidates)
                .expect("the drifted set still resolves a linear chain");
        let error = reloaded
            .verify_live_chain(&live_chain)
            .expect_err("a chain re-derived from disk that drifted from the plan must refuse");
        assert!(
            matches!(error, CohortError::EMigrationPlanDrift { .. }),
            "expected EMigrationPlanDrift from the persisted-file re-derive path, got {error:?}"
        );
        drop(restore);
    }
}
