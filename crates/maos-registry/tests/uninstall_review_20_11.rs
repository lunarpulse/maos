//! Story 20-11 P5: an ordinary partial remove_dir_all failure must not publish a torn generation.
#![cfg(unix)]
#![forbid(unsafe_code)]

use maos_registry::storage::LocalFsRegistryStorage;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

struct Home(Option<std::ffi::OsString>);
impl Drop for Home {
    fn drop(&mut self) {
        match self.0.take() {
            Some(home) => std::env::set_var("HOME", home),
            None => std::env::remove_var("HOME"),
        }
    }
}
static HOME_SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn files(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    fn visit(root: &Path, directory: &Path, output: &mut Vec<(std::path::PathBuf, Vec<u8>)>) {
        for entry in std::fs::read_dir(directory).expect("read generation") {
            let entry = entry.expect("generation entry");
            if entry.file_type().expect("entry type").is_dir() {
                visit(root, &entry.path(), output);
            } else {
                output.push((
                    entry
                        .path()
                        .strip_prefix(root)
                        .expect("generation prefix")
                        .to_owned(),
                    std::fs::read(entry.path()).expect("generation file"),
                ));
            }
        }
    }
    let mut output = Vec::new();
    visit(root, root, &mut output);
    output.sort_by(|left, right| left.0.cmp(&right.0));
    output
}

#[test]
fn review_p5_partial_generation_delete_restores_the_complete_preimage() {
    let _serial = HOME_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let directory = tempfile::tempdir().expect("isolated registry");
    let _home = Home(std::env::var_os("HOME"));
    std::env::set_var("HOME", directory.path());
    let root = directory.path().join(".local/share/maos/registry");
    let generation = root.join("installed/review-spirit/1.0.0");
    std::fs::create_dir_all(&generation).expect("generation");
    std::fs::write(generation.join("manifest.toml"), b"name='review-spirit'\n").expect("manifest");
    for number in 0..64 {
        let child = generation.join(format!("part-{number:03}"));
        std::fs::create_dir(&child).expect("artifact directory");
        std::fs::write(
            child.join("artifact.bin"),
            format!("complete signed generation part {number}"),
        )
        .expect("artifact");
    }
    // chmod does not change directory-entry order. Choosing its last directory
    // makes the genuine EACCES occur after earlier generation files were deleted.
    let blocker = std::fs::read_dir(&generation)
        .expect("directory order")
        .map(|entry| entry.expect("entry"))
        .filter(|entry| entry.file_type().expect("entry type").is_dir())
        .last()
        .expect("artifact directory")
        .path();
    std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o500))
        .expect("real delete denial");
    let before = files(&generation);
    std::fs::write(
        root.join("installed-index.json"),
        br#"{"review-spirit":"review-spirit/1.0.0"}"#,
    )
    .expect("boot index");
    let mut pending = LocalFsRegistryStorage::uninstall_package("review-spirit")
        .expect("stage package")
        .expect("installed generation");
    let result = pending.delete_generation();
    // Restore the fixture permission before asserting, so failures do not leak a tempdir.
    std::fs::set_permissions(&blocker, std::fs::Permissions::from_mode(0o700))
        .expect("restore fixture permission");
    assert!(
        result.is_err(),
        "the permission fixture must cause a genuine remove_dir_all failure"
    );
    let after = files(&generation);
    assert_eq!(
        after, before,
        "failed deletion re-advertised a partially destroyed generation"
    );
    let installed = LocalFsRegistryStorage::default_installed_manifests().expect("boot inventory");
    assert_eq!(
        installed,
        vec![("review-spirit".into(), generation.join("manifest.toml"))]
    );
}

#[test]
fn review_p5_index_recovery_error_is_returned_not_only_logged() {
    let _serial = HOME_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let directory = tempfile::tempdir().expect("isolated recovery error");
    let _home = Home(std::env::var_os("HOME"));
    std::env::set_var("HOME", directory.path());
    let root = directory.path().join(".local/share/maos/registry");
    let generation = root.join("installed/review-spirit/1.0.0");
    std::fs::create_dir_all(&generation).expect("generation");
    std::fs::write(generation.join("manifest.toml"), b"name='review-spirit'\n").expect("manifest");
    std::fs::write(
        generation.join("artifact.bin"),
        b"complete recovery preimage",
    )
    .expect("artifact");
    let before = files(&generation);
    std::fs::write(
        root.join("installed-index.json"),
        br#"{"review-spirit":"review-spirit/1.0.0"}"#,
    )
    .expect("boot index");
    let mut pending = LocalFsRegistryStorage::uninstall_package("review-spirit")
        .expect("stage package")
        .expect("installed generation");
    pending
        .delete_generation()
        .expect("real completed generation deletion");
    struct Permissions(std::path::PathBuf, std::fs::Permissions);
    impl Drop for Permissions {
        fn drop(&mut self) {
            std::fs::set_permissions(&self.0, self.1.clone()).expect("restore fixture permissions");
        }
    }
    let permissions = Permissions(
        root.clone(),
        std::fs::metadata(&root).expect("registry").permissions(),
    );
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o500))
        .expect("real index restoration denial");
    let error = pending
        .rollback()
        .expect_err("index recovery errors must reach the caller");
    assert!(error.to_string().contains("Permission denied"), "{error}");
    assert_eq!(
        files(&generation),
        before,
        "restore every byte before advertising the generation"
    );
    assert!(
        LocalFsRegistryStorage::default_installed_manifests()
            .expect("still-readable boot inventory")
            .is_empty(),
        "failed index recovery must not advertise a partially restored generation"
    );
    assert!(
        pending.generation().is_some(),
        "a failed recovery must retain the complete preimage"
    );
    drop(permissions);
    // This is an explicit recovery operation after fixing the real filesystem
    // fault, not a rerun of a failed test or a retry-to-green.
    pending
        .rollback()
        .expect("operator-directed recovery after permission repair");
    assert_eq!(
        LocalFsRegistryStorage::default_installed_manifests().expect("recovered inventory"),
        vec![("review-spirit".into(), generation.join("manifest.toml"))]
    );
}
