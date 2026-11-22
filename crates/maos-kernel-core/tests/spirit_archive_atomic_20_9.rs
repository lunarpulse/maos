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
fn archive_write_leaves_an_unowned_stale_temp_untouched_and_owns_no_temp() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = SpiritArchive::new(tmp.path().to_path_buf());
    let dir = archive.archive_dir("s", "2.0");
    fs::create_dir_all(&dir).unwrap();
    // A pre-existing file this write did not create is not owned by it: it must be neither
    // taken over nor deleted (a fixed-name temp implementation would clobber/remove it).
    fs::write(dir.join("manifest.tmp"), "half-written garbage").unwrap();
    archive.write("s", "2.0", "[manifest]", b"state").unwrap();
    assert_eq!(fs::read(dir.join("manifest.toml")).unwrap(), b"[manifest]");
    assert_eq!(fs::read(dir.join("snapshot.cbor")).unwrap(), b"state");
    assert_eq!(
        fs::read(dir.join("manifest.tmp")).unwrap(),
        b"half-written garbage",
        "the unowned stale file is untouched"
    );
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["manifest.tmp", "manifest.toml", "snapshot.cbor"],
        "no temp owned by this write survives"
    );
}

#[test]
fn concurrent_writers_never_share_a_temp_and_leave_a_complete_record() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = std::sync::Arc::new(SpiritArchive::new(tmp.path().to_path_buf()));
    let handles: Vec<_> = (0..8)
        .map(|i| {
            let archive = std::sync::Arc::clone(&archive);
            std::thread::spawn(move || {
                for round in 0..8 {
                    let manifest = format!("manifest-{i}-{round}");
                    let snapshot = format!("snapshot-{i}-{round}");
                    archive
                        .write("s", "3.0", &manifest, snapshot.as_bytes())
                        .unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let dir = archive.archive_dir("s", "3.0");
    // Each file must be exactly one complete input actually written; no claim is made that the
    // two files come from the same write (two-file transactional pairing is out of scope).
    let manifest = fs::read_to_string(dir.join("manifest.toml")).unwrap();
    let inputs: Vec<(String, String)> = (0..8)
        .flat_map(|i| {
            (0..8).map(move |r| (format!("manifest-{i}-{r}"), format!("snapshot-{i}-{r}")))
        })
        .collect();
    assert!(
        inputs.iter().any(|(m, _)| *m == manifest),
        "manifest is one complete input: {manifest}"
    );
    let snapshot = fs::read_to_string(dir.join("snapshot.cbor")).unwrap();
    assert!(
        inputs.iter().any(|(_, s)| *s == snapshot),
        "snapshot is one complete input: {snapshot}"
    );
    let mut names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["manifest.toml", "snapshot.cbor"],
        "no temp survives"
    );
}

#[test]
fn failed_write_propagates_and_removes_its_temp() {
    let tmp = tempfile::tempdir().unwrap();
    let archive = SpiritArchive::new(tmp.path().to_path_buf());
    let dir = archive.archive_dir("s", "4.0");
    fs::create_dir_all(&dir).unwrap();
    // A directory at the destination makes the rename fail after the temp is written.
    fs::create_dir(dir.join("manifest.toml")).unwrap();
    assert!(archive.write("s", "4.0", "m", b"state").is_err());
    let temps: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "tmp"))
        .collect();
    assert!(temps.is_empty(), "the owned temp is cleaned on failure");
}
