//! Filesystem discovery — Story 7.4 AC2.

use std::fs;
use std::path::PathBuf;

use maos_skill::{discover_skills, discover_skills_detailed, SkillAdmissionState};

fn write_skill(dir: &std::path::Path, file: &str, id: &str, version: &str) {
    let src = format!(
        "---\nid = \"{id}\"\nversion = \"{version}\"\nname = \"{id}\"\ndescription = \"d\"\n---\nBody for {id}.\n"
    );
    fs::write(dir.join(file), src).unwrap();
}

#[test]
fn discovers_multiple_skills_from_a_temp_root() {
    let tmp = tempfile::tempdir().unwrap();
    write_skill(tmp.path(), "a.md", "skill.a", "1.0.0");
    write_skill(tmp.path(), "b.md", "skill.b", "2.0.0");
    // A non-md file is ignored.
    fs::write(tmp.path().join("notes.txt"), "ignore me").unwrap();

    let roots = vec![tmp.path().to_path_buf()];
    let discovered = discover_skills(&roots);
    assert_eq!(discovered.len(), 2);
    // All discovered skills land Pending — discovery never auto-admits.
    assert!(discovered
        .iter()
        .all(|d| d.state == SkillAdmissionState::Pending));
    let ids: Vec<&str> = discovered.iter().map(|d| d.skill.manifest.id.as_str()).collect();
    assert!(ids.contains(&"skill.a"));
    assert!(ids.contains(&"skill.b"));
}

#[test]
fn malformed_skill_is_skipped_with_observable_reason() {
    let tmp = tempfile::tempdir().unwrap();
    write_skill(tmp.path(), "good.md", "skill.good", "1.0.0");
    // Malformed: unknown field → must be skipped, NOT abort discovery.
    fs::write(
        tmp.path().join("bad.md"),
        "---\nid = \"bad\"\nversion = \"1.0.0\"\nname = \"Bad\"\ndescription = \"d\"\nbogus = 1\n---\nbody\n",
    )
    .unwrap();

    let roots = vec![tmp.path().to_path_buf()];
    let outcome = discover_skills_detailed(&roots);
    assert_eq!(outcome.discovered.len(), 1, "one good skill discovered");
    assert_eq!(outcome.discovered[0].skill.manifest.id, "skill.good");
    assert_eq!(outcome.skipped.len(), 1, "one malformed skill skipped");
    assert!(outcome.skipped[0].0.ends_with("bad.md"));
}

#[test]
fn missing_root_is_not_an_error() {
    let roots = vec![PathBuf::from("/nonexistent/maos/skills/path/zzz")];
    let outcome = discover_skills_detailed(&roots);
    assert!(outcome.discovered.is_empty());
    assert!(outcome.skipped.is_empty());
}

#[test]
fn discovery_is_deterministic_sorted_by_path() {
    let tmp = tempfile::tempdir().unwrap();
    write_skill(tmp.path(), "c.md", "skill.c", "1.0.0");
    write_skill(tmp.path(), "a.md", "skill.a", "1.0.0");
    write_skill(tmp.path(), "b.md", "skill.b", "1.0.0");
    let roots = vec![tmp.path().to_path_buf()];
    let d1 = discover_skills(&roots);
    let d2 = discover_skills(&roots);
    let paths1: Vec<_> = d1.iter().map(|d| d.source_path.clone()).collect();
    let paths2: Vec<_> = d2.iter().map(|d| d.source_path.clone()).collect();
    assert_eq!(paths1, paths2, "discovery order must be deterministic");
    let mut sorted = paths1.clone();
    sorted.sort();
    assert_eq!(paths1, sorted, "discovery order must be sorted by path");
}
