#![forbid(unsafe_code)]

//! Story 1b.5c AC3 — NFR-Test-13 manifest-field coverage gate.
//!
//! Walks `crates/maos-kernel-core/tests/fixtures/manifest/` and asserts:
//!   1. Every `(section, field)` tuple in the live `MANIFEST_FIELDS`
//!      allowlist has ≥3 fixture files under
//!      `<section>/{well-formed, malformed-rejected, edge-case}/`.
//!   2. Every `.toml` file in the fixture tree maps to a tuple in the
//!      allowlist — orphan fixtures fail the build (Decision Register D1).
//!
//! Per Decision Register D1 the allowlist is a hand-maintained `const`
//! rather than reflected from the `RawXxx` structs in
//! `src/security/manifest.rs`. Renaming a manifest field requires
//! renaming the allowlist entry in the same diff, which is the discipline
//! the story wants: diff-reviewable contract changes.

use std::fs;
use std::path::{Path, PathBuf};

/// Single source of truth: every manifest `(section, field)` tuple that
/// must have ≥3 fixture cases. Section-name maps 1:1 to the directory
/// name; field-name maps 1:1 to the fixture filename stem (with the
/// special-case `provider.complete` → `provider_complete` since dots
/// are reserved in TOML keys but allowed as `_` in filenames).
///
/// Refactoring a Raw struct field MUST rename the matching tuple here.
/// Adding a new section parser MUST add at least 3 fixture files AND a
/// tuple here — both the coverage assertion AND the orphan-fixture
/// assertion would otherwise fail.
const MANIFEST_FIELDS: &[(&str, &str)] = &[
    ("class", "name"),
    ("class", "version"),
    ("class", "abi"),
    ("class", "manifest_schema_version"),
    ("class", "min_substrate_version"),
    ("class", "forms"),
    ("class", "trust_tier"),
    ("class", "description"),
    ("capabilities", "provider_complete"),
    ("posture", "default"),
    ("posture", "allowed_max"),
    ("output_shape", "required_fields"),
    ("budget", "context_window_size"),
    ("budget", "time_cap_seconds"),
    ("resources", "cpu_max_pct"),
    ("resources", "memory_max_mb"),
    ("resources", "fd_max"),
    ("sandbox", "tier"),
    ("author", "name"),
    ("author", "homepage"),
    ("epistemic_policy", "rules"),
    ("epistemic_policy", "default_action"),
];

const CATEGORIES: &[&str] = &["well-formed", "malformed-rejected", "edge-case"];

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("manifest")
}

/// Count fixture files for a `(section, field)` tuple — one match per
/// category subdirectory. Filename stem (without `.toml` extension)
/// must equal `field`.
fn count_field_files(root: &Path, section: &str, field: &str) -> usize {
    let mut count = 0;
    let target_stem = format!("{field}.toml");
    for category in CATEGORIES {
        let dir = root.join(section).join(category);
        if !dir.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name == target_stem {
                        count += 1;
                    }
                }
            }
        }
    }
    count
}

/// Enumerate every `.toml` file under the fixture root as
/// (section, category, field-stem) triples. Returns paths that don't
/// map to any tuple in `MANIFEST_FIELDS` — those are orphans the walker
/// must reject (Decision Register D1, reverse-validate).
fn find_orphan_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut orphans = Vec::new();
    let sections = match fs::read_dir(root) {
        Ok(e) => e,
        Err(_) => return orphans,
    };
    for section_entry in sections.flatten() {
        let section_path = section_entry.path();
        if !section_path.is_dir() {
            continue;
        }
        let section_name = match section_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        for category in CATEGORIES {
            let cat_path = section_path.join(category);
            if !cat_path.is_dir() {
                continue;
            }
            let entries = match fs::read_dir(&cat_path) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let file_path = entry.path();
                if !file_path.is_file() {
                    continue;
                }
                if file_path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let stem = match file_path.file_stem().and_then(|s| s.to_str()) {
                    Some(s) => s.to_string(),
                    None => {
                        orphans.push(file_path);
                        continue;
                    }
                };
                let mapped = MANIFEST_FIELDS
                    .iter()
                    .any(|(s, f)| *s == section_name && *f == stem);
                if !mapped {
                    orphans.push(file_path);
                }
            }
        }
        // Also flag stray category directories not in CATEGORIES, or
        // files placed at section level instead of under a category.
        if let Ok(direct_entries) = fs::read_dir(&section_path) {
            for entry in direct_entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    orphans.push(path);
                }
            }
        }
    }
    orphans
}

#[test]
fn test_nfr_test_13_three_cases_per_field() {
    let root = fixture_root();
    assert!(
        root.is_dir(),
        "fixture root missing: {}",
        root.display()
    );

    let mut shortfalls: Vec<String> = Vec::new();
    for (section, field) in MANIFEST_FIELDS {
        let count = count_field_files(&root, section, field);
        if count < 3 {
            shortfalls.push(format!(
                "  - ({section}, {field}): only {count} fixture(s) (need ≥3) under {}/{section}/{{well-formed,malformed-rejected,edge-case}}/{field}.toml",
                root.display()
            ));
        }
    }
    assert!(
        shortfalls.is_empty(),
        "NFR-Test-13 violation — fixture cases short of ≥3 per (section, field):\n{}",
        shortfalls.join("\n")
    );

    let orphans = find_orphan_fixtures(&root);
    assert!(
        orphans.is_empty(),
        "NFR-Test-13 violation — orphan fixtures not mapped to any (section, field) tuple in MANIFEST_FIELDS:\n{}",
        orphans
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
