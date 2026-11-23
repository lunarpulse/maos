//! Story 22-1 AC6 (D-22-1-10 A) — one precedence for the Host's data stores.

use maos_domain::data_root::{resolve_in, roots_from};
use std::ffi::OsStr;
use std::path::PathBuf;

fn roots(maos_home: Option<&str>, xdg: Option<&str>, home: Option<&str>) -> Vec<PathBuf> {
    roots_from(
        maos_home.map(OsStr::new),
        xdg.map(OsStr::new),
        home.map(OsStr::new),
    )
}

#[test]
fn explicit_maos_home_outranks_xdg_outranks_the_legacy_home() {
    assert_eq!(
        roots(Some("/m"), Some("/x"), Some("/h")),
        [
            PathBuf::from("/m"),
            PathBuf::from("/x/maos"),
            PathBuf::from("/h/.local/share/maos")
        ]
    );
    // Empty values are unset; the `$HOME/.maos` fallback of the door rule is NOT a data root.
    assert_eq!(
        roots(Some(""), Some(""), Some("/h")),
        [PathBuf::from("/h/.local/share/maos")]
    );
    assert_eq!(roots(None, None, None), [PathBuf::from("/var/lib/maos")]);
}

#[test]
fn a_new_store_goes_under_the_highest_root() {
    let ranked = roots(
        Some("/nonexistent-m"),
        Some("/nonexistent-x"),
        Some("/nonexistent-h"),
    );
    for rel in [
        "memory",
        "registry",
        "registry/yank_cursor.json",
        "upgrade-plans",
    ] {
        assert_eq!(
            resolve_in(&ranked, rel),
            PathBuf::from("/nonexistent-m").join(rel)
        );
    }
}

#[test]
fn data_under_a_legacy_root_stays_readable_in_place_with_maos_home_set() {
    let scratch = tempfile::tempdir().unwrap();
    let home = scratch.path().join("home");
    let legacy_memory = home.join(".local/share/maos/memory");
    std::fs::create_dir_all(&legacy_memory).unwrap();
    std::fs::write(legacy_memory.join("entry"), b"legacy").unwrap();
    let explicit = scratch.path().join("explicit");
    std::fs::create_dir_all(&explicit).unwrap();

    let ranked = roots_from(Some(explicit.as_os_str()), None, Some(home.as_os_str()));
    let memory = resolve_in(&ranked, "memory");
    assert_eq!(
        memory, legacy_memory,
        "legacy data is used in place, not shadowed"
    );
    assert_eq!(std::fs::read(memory.join("entry")).unwrap(), b"legacy");
    // A store that exists nowhere is created under the explicit root.
    assert_eq!(resolve_in(&ranked, "registry"), explicit.join("registry"));
    // Once the explicit root holds the store, it wins.
    std::fs::create_dir_all(explicit.join("memory")).unwrap();
    assert_eq!(resolve_in(&ranked, "memory"), explicit.join("memory"));
}
