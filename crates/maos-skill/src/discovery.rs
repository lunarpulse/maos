//! Filesystem skill discovery.
//!
//! Scans the conventional `[skills.search_path]` roots (architecture §5:69-70 —
//! `~/.maos/skills/`, `_bmad/skills/`, `/usr/share/maos/skills/`) for `*.md`
//! skill files, parses each, and returns the discovered skills + their initial
//! `Pending` admission state. A malformed skill file is SKIPPED (discovery does
//! NOT abort on one bad file) with a `tracing::warn!` AND recorded in the
//! observable `skipped` companion.

use std::path::{Path, PathBuf};

use crate::admission::SkillAdmissionState;
use crate::errors::ESkillSchema;
use crate::schema::{parse_skill, Skill};

/// A skill discovered on the filesystem, with its source path and initial
/// admission state (always `Pending` — discovery never auto-admits).
#[derive(Debug, Clone)]
pub struct DiscoveredSkill {
    /// The parsed, validated skill.
    pub skill: Skill,
    /// The `*.md` file it was parsed from.
    pub source_path: PathBuf,
    /// Initial admission state — always `Pending` (FR39).
    pub state: SkillAdmissionState,
}

/// The full outcome of a discovery pass: the discovered skills + the observable
/// list of files skipped (with the `ESkillSchema` reason they were skipped).
#[derive(Debug, Default)]
pub struct DiscoveryOutcome {
    /// Successfully parsed + validated skills.
    pub discovered: Vec<DiscoveredSkill>,
    /// Files skipped because they failed to parse/validate, with the reason.
    pub skipped: Vec<(PathBuf, ESkillSchema)>,
}

/// The conventional skill search-path roots (architecture §5:69-70). The kernel
/// passes these from config defaults; `~` is expanded to `$HOME`.
pub fn default_search_path() -> Vec<PathBuf> {
    [
        "~/.maos/skills/",
        "_bmad/skills/",
        "/usr/share/maos/skills/",
    ]
    .iter()
    .map(|r| expand_tilde(r))
    .collect()
}

/// Expand a leading `~` to `$HOME` (no-op if `$HOME` is unset or no leading `~`).
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return Path::new(&home).join(rest);
        }
    } else if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home);
        }
    }
    PathBuf::from(path)
}

/// Discover skills under `roots`, returning only the successfully-parsed skills.
/// Malformed files are skipped with a `tracing::warn!` (use
/// [`discover_skills_detailed`] to also collect the skip reasons).
pub fn discover_skills(roots: &[PathBuf]) -> Vec<DiscoveredSkill> {
    discover_skills_detailed(roots).discovered
}

/// Discover skills under `roots`, returning both the discovered skills AND the
/// observable list of skipped files with their `ESkillSchema` reasons.
///
/// **Flat-only scan:** each root is scanned with `read_dir` (top-level `*.md`
/// files only). Subdirectories are NOT recursed — skills must reside directly
/// under the configured search-path roots. This is a v0.5 constraint; recursive
/// discovery may be introduced when the spec clarifies nested-directory semantics.
pub fn discover_skills_detailed(roots: &[PathBuf]) -> DiscoveryOutcome {
    let mut outcome = DiscoveryOutcome::default();

    // Collect candidate `*.md` paths deterministically (sorted) across all roots.
    let mut candidates: Vec<PathBuf> = Vec::new();
    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            // Missing root is not an error — operators need not create all three.
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") && path.is_file() {
                candidates.push(path);
            }
        }
    }
    candidates.sort();

    for path in candidates {
        let src = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "skill discovery: unreadable file skipped");
                outcome
                    .skipped
                    .push((path, ESkillSchema::TomlParse(format!("io: {e}"))));
                continue;
            }
        };
        match parse_skill(&src) {
            Ok(skill) => outcome.discovered.push(DiscoveredSkill {
                skill,
                source_path: path,
                state: SkillAdmissionState::Pending,
            }),
            Err(reason) => {
                tracing::warn!(path = %path.display(), reason = %reason, "skill discovery: malformed skill skipped");
                outcome.skipped.push((path, reason));
            }
        }
    }

    outcome
}
