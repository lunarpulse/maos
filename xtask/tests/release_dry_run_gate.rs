//! Story 15-4 AC1 — behavioral vectors for the release artifact denominator.
//!
//! These tests drive the same manifest writer used by `xtask release-dry-run`.
//! Missing declared artifacts must fail before hashes are trusted; unrelated
//! packaging files and symlinks remain forward-compatible for Epic 20.

use std::fs;
use std::path::Path;

use maos_audit::release_verify::parse_sha256sums;
use tempfile::TempDir;
use xtask::release_dry_run::{clean_staged_release_outputs, generate_manifest, ARTIFACTS};

const HOST_TARGET: &str = "x86_64-unknown-linux-gnu";
const ARM_TARGET: &str = "aarch64-unknown-linux-gnu";

fn write_artifacts(dir: &Path, target: &str) {
    let suffix = match target {
        HOST_TARGET => "linux-amd64",
        ARM_TARGET => "linux-arm64",
        other => panic!("unsupported fixture target: {other}"),
    };
    for artifact in ARTIFACTS {
        fs::write(
            dir.join(format!("{}-{suffix}", artifact.binary)),
            format!("{}-{target}", artifact.binary),
        )
        .expect("write artifact fixture");
    }
}

/// Known-answer digests over the fixture bytes `write_artifacts` stages for
/// `HOST_TARGET`, computed independently (`printf '<content>' | sha256sum`). A
/// manifest that carries the right number of lines with constant or swapped hashes
/// must not pass: only the `maos-<suffix>` entry is re-verified by the discipline
/// self-test, so a wrong `maosctl` line would otherwise survive to the first tag.
const HOST_DIGESTS: [(&str, &str); 2] = [
    (
        "maos-linux-amd64",
        "35bf7c30d75be5349b37cc63b25e28ad68ba5f17416ce70d2404d89eb63db310",
    ),
    (
        "maosctl-linux-amd64",
        "ddad3727c93b3dbf8fae551c54b842423be6d0972c12ede86c775c64c97a716d",
    ),
];

#[test]
fn manifest_denominator_matches_declared_artifact_count_before_hash_checks() {
    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);

    let report = generate_manifest(temp.path(), &[HOST_TARGET]).expect("manifest should generate");
    let content = fs::read_to_string(temp.path().join("SHA256SUMS")).expect("manifest");
    let entries = parse_sha256sums(&content).expect("manifest must round-trip through parser");

    assert_eq!(report.artifact_count, ARTIFACTS.len());
    assert_eq!(entries.len(), report.artifact_count);
    assert_eq!(content.lines().count(), ARTIFACTS.len());

    for (filename, digest) in HOST_DIGESTS {
        let entry = entries
            .iter()
            .find(|entry| entry.filename == filename)
            .unwrap_or_else(|| panic!("manifest must cover {filename}: {content}"));
        assert_eq!(entry.hash, digest, "digest for {filename}");
    }
}

/// A truncated `download-artifact` payload must not become a signed release that
/// verifies green. `release-verify --verify` only checks manifest entries against
/// disk, so a zero-byte binary would pass every downstream control.
#[test]
fn zero_length_declared_artifact_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);
    fs::write(temp.path().join("maosctl-linux-amd64"), b"").expect("truncate fixture");

    let error = generate_manifest(temp.path(), &[HOST_TARGET]).expect_err("empty artifact reds");
    assert!(error.contains("maosctl-linux-amd64"), "error: {error}");
    assert!(error.contains("empty"), "error: {error}");
}

/// Regenerating a manifest must not leave the previous signature beside it: the
/// pair would verify as a consistent release while covering different bytes.
#[test]
fn regenerating_the_manifest_drops_the_previous_signature() {
    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);
    generate_manifest(temp.path(), &[HOST_TARGET]).expect("first manifest");
    fs::write(temp.path().join("SHA256SUMS.sig"), b"stale-signature").expect("stale sig");

    generate_manifest(temp.path(), &[HOST_TARGET]).expect("second manifest");

    assert!(!temp.path().join("SHA256SUMS.sig").exists());
    assert!(temp.path().join("SHA256SUMS").exists());
}

#[test]
fn missing_binary_target_pair_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);
    fs::remove_file(temp.path().join("maosctl-linux-amd64")).expect("remove fixture");

    let error =
        generate_manifest(temp.path(), &[HOST_TARGET]).expect_err("missing artifact must red");
    assert!(
        error.contains("maosctl-linux-amd64") && error.contains("missing"),
        "error must name the missing pair: {error}"
    );
    assert!(!temp.path().join("SHA256SUMS").exists());
}

#[test]
fn declared_artifact_outside_requested_targets_is_refused() {
    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);
    write_artifacts(temp.path(), ARM_TARGET);

    let error = generate_manifest(temp.path(), &[HOST_TARGET])
        .expect_err("declared artifact omitted by target selection must red");
    assert!(
        error.contains("maos-linux-arm64") && error.contains("not covered"),
        "error must name the uncovered declared artifact: {error}"
    );
}

#[test]
fn regular_build_cleanup_removes_stale_release_outputs_only() {
    let temp = TempDir::new().expect("tempdir");
    for filename in [
        "maos-linux-arm64",
        "SHA256SUMS",
        "SHA256SUMS.sig",
        "maos.deb",
    ] {
        fs::write(temp.path().join(filename), b"stale").expect("write fixture");
    }

    clean_staged_release_outputs(temp.path()).expect("stale outputs clean");

    assert!(!temp.path().join("maos-linux-arm64").exists());
    assert!(!temp.path().join("SHA256SUMS").exists());
    assert!(!temp.path().join("SHA256SUMS.sig").exists());
    assert!(temp.path().join("maos.deb").exists());
}

#[cfg(unix)]
#[test]
fn packaging_file_and_symlink_do_not_expand_the_declared_denominator() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().expect("tempdir");
    write_artifacts(temp.path(), HOST_TARGET);
    fs::write(temp.path().join("maos_0.1.0_amd64.deb"), b"future package")
        .expect("write package fixture");
    symlink("maos-linux-amd64", temp.path().join("maos-current")).expect("write symlink fixture");

    let report = generate_manifest(temp.path(), &[HOST_TARGET])
        .expect("unrelated package and symlink must remain compatible");
    let content = fs::read_to_string(temp.path().join("SHA256SUMS")).expect("manifest");
    let entries = parse_sha256sums(&content).expect("parse manifest");

    assert_eq!(report.artifact_count, ARTIFACTS.len());
    assert_eq!(entries.len(), ARTIFACTS.len());
    assert!(entries
        .iter()
        .all(|entry| !entry.filename.ends_with(".deb")));
    assert!(entries.iter().all(|entry| entry.filename != "maos-current"));
}
