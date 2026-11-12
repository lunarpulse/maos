#![forbid(unsafe_code)]

//! Story 20-9 (GLM r3 F3) — the FR11 archive record is crash-atomic: a write replaces the
//! file (temp file in the same directory, fsync, rename, directory fsync), so a death never
//! leaves a partial `manifest.toml` that write-if-absent would never repair.

use maos_kernel_core::hot_swap::SpiritArchive;
use std::fs;

#[cfg(unix)]
#[test]
fn archive_overwrite_never_mutates_the_previous_inode() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = SpiritArchive::new(tmp.path().to_path_buf());
    archive
        .write("s", "1.0", "old-manifest", b"old-snapshot")
        .unwrap();
    let dir = archive.archive_dir("s", "1.0");
    let held = dir.join("held-manifest");
    fs::hard_link(dir.join("manifest.toml"), &held).unwrap();
    archive
        .write("s", "1.0", "new-manifest", b"new-snapshot")
        .unwrap();
    assert_eq!(
        fs::read_to_string(&held).unwrap(),
        "old-manifest",
        "a write replaces the file, it never rewrites the old inode in place"
    );
    assert_eq!(
        fs::read_to_string(dir.join("manifest.toml")).unwrap(),
        "new-manifest"
    );
    assert_eq!(
        archive.read("s", "1.0").unwrap(),
        Some(b"new-snapshot".to_vec())
    );
}

#[test]
fn archive_write_survives_a_stale_temp_file_and_leaves_none() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = SpiritArchive::new(tmp.path().to_path_buf());
    let dir = archive.archive_dir("s", "2.0");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.tmp"), "half-written garbage").unwrap();
    archive.write("s", "2.0", "[manifest]", b"state").unwrap();
    assert_eq!(
        fs::read_to_string(dir.join("manifest.toml")).unwrap(),
        "[manifest]"
    );
    let leftovers: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "tmp"))
        .collect();
    assert!(leftovers.is_empty(), "no temp file survives a write");
}
