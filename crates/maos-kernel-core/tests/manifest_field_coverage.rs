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

use maos_domain::invariants::i1::Scope;
use maos_kernel_core::security::{capabilities_required_to_scopes, CapabilitiesRequired};
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
    ("capabilities", "loom_read"),
    ("capabilities", "loom_write"),
    ("capabilities", "loom_scan"),
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
    ("scheduling", "priority_weight"),
    ("lifecycle", "enabled_hooks"),
    ("hot_swap", "state_schema_uri"),
    ("hot_swap", "state_schema_version"),
    ("migrates_from", "versions"),
    ("halt_protocol_compatibility", "version"),
    ("providers", "primary-anthropic-no-fallback"),
    ("providers", "primary-openai-with-anthropic-fallback"),
    ("providers", "primary-ollama-air-gapped"),
    ("providers", "bad-pin"),
    ("providers", "empty-endpoint"),
    ("providers", "unsupported-id"),
    ("sandbox", "tier-t3"),
    ("sandbox", "tier-t3-with-pin"),
    ("sandbox", "image-pin-missing"),
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

fn capability_fixture_paths(root: &Path, category: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(root.join("capabilities").join(category))
        .expect("capabilities fixture category must be readable")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("toml"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn capabilities_fixtures_deserialize_to_their_declared_outcomes() {
    let root = fixture_root();
    let mut fixture_count = 0;
    for category in CATEGORIES {
        for path in capability_fixture_paths(&root, category) {
            fixture_count += 1;
            let fixture = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            let manifest_fragment = if fixture.contains("provider.complete") {
                fixture
            } else {
                format!("provider.complete = [\"anthropic.default\"]\n{fixture}")
            };
            let parsed = CapabilitiesRequired::from_toml_str(&manifest_fragment);
            if *category == "malformed-rejected" {
                assert!(parsed.is_err(), "{} must be rejected", path.display());
                continue;
            }
            let caps = parsed.unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            if *category == "edge-case" {
                match path.file_stem().and_then(|stem| stem.to_str()) {
                    Some("loom_read") => {
                        assert!(!capabilities_required_to_scopes(&caps).contains(&Scope::LoomRead))
                    }
                    Some("loom_write") => {
                        assert!(!capabilities_required_to_scopes(&caps).contains(&Scope::LoomWrite))
                    }
                    Some("loom_scan") => {
                        assert!(!capabilities_required_to_scopes(&caps).contains(&Scope::LoomScan))
                    }
                    _ => {}
                }
            }
        }
    }
    assert_eq!(
        fixture_count, 12,
        "every capabilities fixture must be exercised"
    );
}

#[test]
fn production_capability_parsers_are_all_schema_degraded() {
    let main_rs = include_str!("../../maos-bin/src/main.rs");
    assert_eq!(
        main_rs
            .matches("CapabilitiesRequired::from_toml_str")
            .count(),
        5,
        "a new production capability parser must add schema degradation coverage"
    );
    assert_eq!(
        main_rs
            .matches(".degrade_for_schema_version(class_section.manifest_schema_version)")
            .count(),
        6,
        "every direct parser and both caps_required_or_empty admission paths must degrade loom"
    );
}

#[test]
fn test_nfr_test_13_three_cases_per_field() {
    let root = fixture_root();
    assert!(root.is_dir(), "fixture root missing: {}", root.display());

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
