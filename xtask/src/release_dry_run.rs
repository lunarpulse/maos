//! Story 15-4 AC1 — build and stage the release artifact set without secrets.
//!
//! The artifact denominator is explicit. Both the local build path and the
//! tagged-release manifest path use [`ARTIFACTS`]; globs never define release
//! contents.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use maos_audit::release_verify::{generate_sha256sums, sha256_hex};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Artifact {
    pub package: &'static str,
    pub binary: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Target {
    triple: &'static str,
    suffix: &'static str,
}

pub const ARTIFACTS: &[Artifact] = &[
    Artifact {
        package: "maos-bin",
        binary: "maos",
    },
    Artifact {
        package: "maos-cli",
        binary: "maosctl",
    },
];

/// Written by the manifest path, never a release artifact. Cleared before a
/// rewrite so a stale signature can never outlive the manifest it signed.
const MANIFEST_OUTPUTS: [&str; 2] = ["SHA256SUMS", "SHA256SUMS.sig"];

const TARGETS: &[Target] = &[
    Target {
        triple: "x86_64-unknown-linux-gnu",
        suffix: "linux-amd64",
    },
    Target {
        triple: "aarch64-unknown-linux-gnu",
        suffix: "linux-arm64",
    },
    Target {
        triple: "aarch64-apple-darwin",
        suffix: "darwin-arm64",
    },
];

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Report {
    pub passed: bool,
    pub manifest_only: bool,
    pub dist_dir: String,
    pub targets: Vec<String>,
    pub artifact_count: usize,
    pub artifacts: Vec<String>,
    pub manifest: String,
}

pub fn run(
    manifest_only: bool,
    dist_dir: &str,
    requested_targets: &[String],
    json: bool,
) -> Result<(), String> {
    let targets = if requested_targets.is_empty() {
        vec![host_target()?]
    } else {
        requested_targets.to_vec()
    };
    let target_refs: Vec<&str> = targets.iter().map(String::as_str).collect();
    let dist_dir = Path::new(dist_dir);

    if !manifest_only {
        build_and_stage(dist_dir, &target_refs)?;
    }
    let mut report = generate_manifest(dist_dir, &target_refs)?;
    report.manifest_only = manifest_only;

    if json {
        println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|error| format!("serialize release-dry-run report: {error}"))?
        );
    } else {
        eprintln!(
            "release-dry-run: PASS — {} artifact(s), {} target(s), manifest {}",
            report.artifact_count,
            report.targets.len(),
            report.manifest,
        );
    }
    Ok(())
}

pub fn generate_manifest(dist_dir: &Path, requested_targets: &[&str]) -> Result<Report, String> {
    fs::create_dir_all(dist_dir)
        .map_err(|error| format!("create dist directory {}: {error}", dist_dir.display()))?;
    let targets = resolve_targets(requested_targets)?;
    let expected = expected_artifacts(&targets);
    refuse_uncovered_declared_artifacts(dist_dir, &expected)?;

    let mut manifest_entries = Vec::with_capacity(expected.len());
    for filename in &expected {
        let path = dist_dir.join(filename);
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("missing declared release artifact {filename}: {error}"))?;
        if !metadata.file_type().is_file() {
            return Err(format!(
                "declared release artifact {filename} is not a regular file"
            ));
        }
        if metadata.len() == 0 {
            return Err(format!("declared release artifact {filename} is empty"));
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("read declared release artifact {filename}: {error}"))?;
        manifest_entries.push((filename.clone(), sha256_hex(&bytes)));
    }

    let manifest = generate_sha256sums(&manifest_entries);
    for stale in MANIFEST_OUTPUTS {
        remove_if_present(&dist_dir.join(stale))?;
    }
    let manifest_path = dist_dir.join("SHA256SUMS");
    fs::write(&manifest_path, manifest)
        .map_err(|error| format!("write {}: {error}", manifest_path.display()))?;

    Ok(Report {
        passed: true,
        manifest_only: true,
        dist_dir: dist_dir.display().to_string(),
        targets: targets
            .iter()
            .map(|target| target.triple.to_string())
            .collect(),
        artifact_count: manifest_entries.len(),
        artifacts: manifest_entries
            .into_iter()
            .map(|(filename, _)| filename)
            .collect(),
        manifest: manifest_path.display().to_string(),
    })
}

fn build_and_stage(dist_dir: &Path, requested_targets: &[&str]) -> Result<(), String> {
    let targets = resolve_targets(requested_targets)?;
    fs::create_dir_all(dist_dir)
        .map_err(|error| format!("create dist directory {}: {error}", dist_dir.display()))?;
    clean_staged_release_outputs(dist_dir)?;

    for target in targets {
        for artifact in ARTIFACTS {
            let mut command = Command::new("cargo");
            command.args([
                "build",
                "--release",
                "--locked",
                // Pinned, not inherited: staging below reads `target/<triple>/release`,
                // and an ambient CARGO_TARGET_DIR would build elsewhere and leave this
                // path holding a stale binary that still hashes and signs green.
                "--target-dir",
                "target",
                "--target",
                target.triple,
                "-p",
                artifact.package,
                "--bin",
                artifact.binary,
            ]);
            if target.triple == "aarch64-unknown-linux-gnu" {
                command.env(
                    "CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER",
                    "aarch64-linux-gnu-gcc",
                );
                command.env("CC_aarch64_unknown_linux_gnu", "aarch64-linux-gnu-gcc");
            }
            let status = command.status().map_err(|error| {
                format!(
                    "launch cargo build for {} on {}: {error}",
                    artifact.binary, target.triple
                )
            })?;
            if !status.success() {
                return Err(format!(
                    "cargo build failed for {} on {} with {status}",
                    artifact.binary, target.triple
                ));
            }

            let source = PathBuf::from("target")
                .join(target.triple)
                .join("release")
                .join(artifact.binary);
            let filename = artifact_filename(artifact.binary, target);
            fs::copy(&source, dist_dir.join(&filename)).map_err(|error| {
                format!(
                    "stage {} as {}: {error}",
                    source.display(),
                    dist_dir.join(&filename).display()
                )
            })?;
        }
    }
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("remove stale {}: {error}", path.display())),
    }
}

pub fn clean_staged_release_outputs(dist_dir: &Path) -> Result<(), String> {
    for filename in expected_artifacts(TARGETS) {
        remove_if_present(&dist_dir.join(filename))?;
    }
    for stale in MANIFEST_OUTPUTS {
        remove_if_present(&dist_dir.join(stale))?;
    }
    Ok(())
}

fn resolve_targets(requested: &[&str]) -> Result<Vec<Target>, String> {
    if requested.is_empty() {
        return Err("at least one release target is required".to_string());
    }
    let mut seen = BTreeSet::new();
    let mut targets = Vec::with_capacity(requested.len());
    for triple in requested {
        let target = TARGETS
            .iter()
            .find(|target| target.triple == *triple)
            .copied()
            .ok_or_else(|| format!("unsupported release target {triple}"))?;
        if !seen.insert(target.triple) {
            return Err(format!("duplicate release target {triple}"));
        }
        targets.push(target);
    }
    Ok(targets)
}

fn expected_artifacts(targets: &[Target]) -> BTreeSet<String> {
    targets
        .iter()
        .flat_map(|target| {
            ARTIFACTS
                .iter()
                .map(move |artifact| artifact_filename(artifact.binary, *target))
        })
        .collect()
}

fn refuse_uncovered_declared_artifacts(
    dist_dir: &Path,
    expected: &BTreeSet<String>,
) -> Result<(), String> {
    let all_declared = expected_artifacts(TARGETS);
    for entry in fs::read_dir(dist_dir)
        .map_err(|error| format!("read dist directory {}: {error}", dist_dir.display()))?
    {
        let entry = entry.map_err(|error| format!("read dist directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("read type for {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        let Some(filename) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if all_declared.contains(&filename) && !expected.contains(&filename) {
            return Err(format!(
                "declared release artifact {filename} is present but not covered by requested targets"
            ));
        }
    }
    Ok(())
}

fn artifact_filename(binary: &str, target: Target) -> String {
    format!("{binary}-{}", target.suffix)
}

fn host_target() -> Result<String, String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .map_err(|error| format!("run rustc -vV to resolve host target: {error}"))?;
    if !output.status.success() {
        return Err(format!("rustc -vV failed with {}", output.status));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("rustc -vV returned non-UTF-8 output: {error}"))?;
    stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_string)
        .ok_or_else(|| "rustc -vV did not report a host target".to_string())
}
