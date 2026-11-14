//! `gate_common::target_dir_from` — the Cargo target directory xtask gates read
//! build output from (rustdoc JSON, `maos` binaries). Pure value arithmetic: no
//! environment mutation, no filesystem.

use std::ffi::OsString;
use std::path::PathBuf;
use xtask::gate_common::target_dir_from;

fn var(value: &str) -> Option<OsString> {
    Some(OsString::from(value))
}

#[test]
fn target_dir_follows_cargo_env_precedence_and_defaults_to_target() {
    assert_eq!(target_dir_from(None, None), PathBuf::from("target"));
    assert_eq!(target_dir_from(var(""), var("")), PathBuf::from("target"));
    assert_eq!(
        target_dir_from(var("/mnt/build/cargo-maos"), None),
        PathBuf::from("/mnt/build/cargo-maos")
    );
    assert_eq!(
        target_dir_from(None, var("build/out")),
        PathBuf::from("build/out")
    );
    assert_eq!(
        target_dir_from(var("/explicit"), var("/fallback")),
        PathBuf::from("/explicit")
    );
    assert_eq!(
        target_dir_from(var(""), var("/fallback")),
        PathBuf::from("/fallback")
    );
}
