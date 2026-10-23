#![forbid(unsafe_code)]

//! Story 20-12 AC2 — `build-test-fixtures`: every binary and WASM artefact a
//! test resolves is built HERE, once, before the suite (a CI step), and listed in
//! `<target>/maos-fixtures/manifest.json` (or `$MAOS_FIXTURE_DIR`, which must be
//! absolute). Tests read it through `tests/harness/fixtures.rs`; no test spawns
//! `cargo build`.
//!
//! Each build is the exact command a test used to run inside itself, into the
//! same artefact location (so production lookups such as "`worker-cli-fixture`
//! beside the `maos` executable" are unchanged). Two deliberate differences:
//! `recall-probe`/`compat-echo-2-0` land in ONE place instead of per-test target
//! dirs, and the `runner-fault-inject` runner is built in the main target dir
//! (sharing its dependency graph) and its uplifted binary is COPIED to
//! `<fixture dir>/maos-wasm-runner-fault-inject` before anything else can
//! re-uplift `<target>/debug/maos-wasm-runner`.
//!
//! **Staleness (review 2026-10-23).** Each manifest entry records `built_at`
//! (unix seconds at the start of its build) and `inputs`: the source roots of
//! every local package in the fixture's normal/build dependency closure (from
//! `cargo metadata`), each package's `Cargo.toml` and `wit/`, the workspace
//! `Cargo.toml` and the root `wit/`. The harness refuses an artefact any of whose
//! `.rs`/`.toml`/`.wit` inputs is newer — a plant in fixture code never runs the
//! old binary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The manifest format the harness parses (`tests/harness/fixtures.rs`).
const SCHEMA: u64 = 2;
const COMMAND: &str = "cargo run -p xtask -- build-test-fixtures";

/// One fixture build: the cargo arguments and the artefacts it produces.
struct Fixture {
    /// `--only` selector; also the first manifest name.
    name: &'static str,
    /// The package built and the manifest `cargo metadata` resolves it from.
    package: &'static str,
    manifest: PathBuf,
    args: Vec<String>,
    env: Vec<(&'static str, &'static str)>,
    /// A built file copied to its published path right after the build.
    copy: Option<(PathBuf, PathBuf)>,
    artefacts: Vec<(&'static str, PathBuf)>,
}

/// One manifest entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub built_at: u64,
    pub inputs: Vec<PathBuf>,
}

fn s(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_owned()).collect()
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The fixture table. `root` is the workspace, `target` the Cargo target dir,
/// `fixture_dir` where copied artefacts land, `release` selects the profile of
/// the profile-coupled workspace binaries.
fn fixtures(root: &Path, target: &Path, fixture_dir: &Path, release: bool) -> Vec<Fixture> {
    let profile = if release { "release" } else { "debug" };
    let profile_flag: &[&str] = if release { &["--release"] } else { &[] };
    let workspace = root.join("Cargo.toml");
    let wasm = target.join("wasm32-wasip2/release");
    let guest = |name: &'static str, artefact: &'static str| {
        let manifest = root.join(format!("crates/maos-wasm-host/guests/{name}/Cargo.toml"));
        Fixture {
            name: artefact,
            package: name,
            args: [
                s(&[
                    "build",
                    "--locked",
                    "--release",
                    "--target",
                    "wasm32-wasip2",
                ]),
                s(&["--manifest-path"]),
                vec![path_arg(&manifest)],
                s(&["--target-dir"]),
                vec![path_arg(target)],
            ]
            .concat(),
            manifest,
            env: Vec::new(),
            copy: None,
            artefacts: vec![(artefact, wasm.join(artefact))],
        }
    };
    let standalone = |name: &'static str, dir: &str, extra: &[&str]| {
        let dir = root.join(dir);
        Fixture {
            name,
            package: "forbidden-syscall-probe",
            manifest: dir.join("Cargo.toml"),
            args: [
                s(&["build", "--release"]),
                s(extra),
                s(&["--manifest-path"]),
                vec![path_arg(&dir.join("Cargo.toml"))],
                s(&["--target-dir"]),
                vec![path_arg(&dir.join("target"))],
            ]
            .concat(),
            env: Vec::new(),
            copy: None,
            artefacts: vec![(name, dir.join("target/release/forbidden-syscall-probe"))],
        }
    };
    let fault_inject = fixture_dir.join("maos-wasm-runner-fault-inject");
    let twin = root.join("crates/maos-wasm-host/guests/equiv-fixture/native-twin/Cargo.toml");
    let mut table = vec![
        Fixture {
            name: "worker-cli-fixture",
            package: "worker",
            manifest: workspace.clone(),
            args: [
                s(&["build", "--locked", "-p", "worker"]),
                s(&[
                    "--bin",
                    "worker-cli-fixture",
                    "--bin",
                    "worker-probe-fixture",
                ]),
                s(profile_flag),
            ]
            .concat(),
            env: Vec::new(),
            copy: None,
            artefacts: vec![
                (
                    "worker-cli-fixture",
                    target.join(profile).join("worker-cli-fixture"),
                ),
                (
                    "worker-probe-fixture",
                    target.join(profile).join("worker-probe-fixture"),
                ),
            ],
        },
        Fixture {
            name: "maosctl",
            package: "maos-cli",
            manifest: workspace.clone(),
            args: [
                s(&["build", "--locked", "-p", "maos-cli", "--bin", "maosctl"]),
                s(profile_flag),
            ]
            .concat(),
            env: Vec::new(),
            copy: None,
            artefacts: vec![("maosctl", target.join(profile).join("maosctl"))],
        },
        Fixture {
            name: "maos-wasm-runner",
            package: "maos-wasm-host",
            manifest: workspace.clone(),
            args: s(&["build", "--locked", "--release", "-p", "maos-wasm-host"])
                .into_iter()
                .chain(s(&["--bin", "maos-wasm-runner"]))
                .collect(),
            env: Vec::new(),
            copy: None,
            artefacts: vec![("maos-wasm-runner", target.join("release/maos-wasm-runner"))],
        },
        Fixture {
            name: "maos-wasm-runner-fault-inject",
            package: "maos-wasm-host",
            manifest: workspace,
            args: [
                s(&["build", "--locked", "-p", "maos-wasm-host"]),
                s(&[
                    "--features",
                    "runner-fault-inject",
                    "--bin",
                    "maos-wasm-runner",
                ]),
            ]
            .concat(),
            env: Vec::new(),
            copy: Some((target.join("debug/maos-wasm-runner"), fault_inject.clone())),
            artefacts: vec![("maos-wasm-runner-fault-inject", fault_inject)],
        },
        guest("echo-spirit", "echo_spirit.wasm"),
        guest("routed-relay", "routed_relay.wasm"),
        guest("recall-probe", "recall_probe.wasm"),
        guest("compat-echo-2-0", "compat_echo_2_0.wasm"),
        Fixture {
            name: "equiv-native-twin",
            package: "equiv-native-twin",
            args: [
                s(&["build", "--locked", "--release", "--manifest-path"]),
                vec![path_arg(&twin)],
                s(&["--target-dir"]),
                vec![path_arg(target)],
            ]
            .concat(),
            manifest: twin,
            env: Vec::new(),
            copy: None,
            artefacts: vec![
                (
                    "equiv-native-twin",
                    target.join("release/equiv-native-twin"),
                ),
                ("hostile-twin", target.join("release/hostile-twin")),
            ],
        },
        standalone(
            "forbidden-syscall-probe",
            "crates/maos-wasm-host/test-fixtures/forbidden-syscall-probe",
            &["--locked"],
        ),
        standalone(
            "escape-forbidden-syscall-probe",
            "crates/maos-escape-detector/test-fixtures/forbidden-syscall-probe",
            &["--locked"],
        ),
    ];
    // The static T3 probe is an x86_64 artefact (`t3_network_escape_executor.rs`
    // reads it at a fixed path); arm runners have no T3 substrate.
    if cfg!(target_arch = "x86_64") {
        let dir = root.join("crates/maos-eval/test-fixtures/t3-network-escape-probe");
        table.push(Fixture {
            name: "t3-network-escape-probe",
            package: "t3-network-escape-probe",
            manifest: dir.join("Cargo.toml"),
            args: [
                s(&["build", "--locked", "--release"]),
                s(&["--target", "x86_64-unknown-linux-gnu", "--manifest-path"]),
                vec![path_arg(&dir.join("Cargo.toml"))],
                s(&["--target-dir"]),
                vec![path_arg(&dir.join("target"))],
            ]
            .concat(),
            env: vec![("RUSTFLAGS", "-C target-feature=+crt-static")],
            copy: None,
            artefacts: vec![(
                "t3-network-escape-probe",
                dir.join("target/x86_64-unknown-linux-gnu/release/t3-network-escape-probe"),
            )],
        });
    }
    table
}

fn metadata(cargo: &str, root: &Path, args: &[&str]) -> Result<serde_json::Value, String> {
    let output = Command::new(cargo)
        .arg("metadata")
        .args(["--format-version", "1"])
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cargo metadata is not JSON: {e}"))
}

/// `cargo metadata`'s `target_directory` — honours `CARGO_TARGET_DIR` and
/// `.cargo/config.toml` exactly as the test build does.
fn target_dir(cargo: &str, root: &Path) -> Result<PathBuf, String> {
    metadata(cargo, root, &["--no-deps"])?["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| "cargo metadata has no target_directory".to_owned())
}

/// The staleness inputs of `package` in a full `cargo metadata` resolve: for
/// every local (path) package in its normal/build dependency closure, its
/// `Cargo.toml`, the source root of each lib/bin target, its build script and
/// its `wit/` dir. Test/bench/example targets are not inputs of the artefact.
fn package_inputs(metadata: &serde_json::Value, package: &str) -> Result<Vec<PathBuf>, String> {
    let empty = Vec::new();
    let packages = metadata["packages"].as_array().unwrap_or(&empty);
    let local = |id: &str| {
        packages
            .iter()
            .find(|p| p["id"].as_str() == Some(id) && p["source"].is_null())
    };
    let root = packages
        .iter()
        .find(|p| p["name"].as_str() == Some(package) && p["source"].is_null())
        .and_then(|p| p["id"].as_str())
        .ok_or_else(|| format!("cargo metadata names no local package `{package}`"))?;
    let nodes = metadata["resolve"]["nodes"].as_array().unwrap_or(&empty);
    let mut seen = BTreeSet::from([root.to_owned()]);
    let mut queue = vec![root.to_owned()];
    while let Some(id) = queue.pop() {
        let Some(node) = nodes.iter().find(|n| n["id"].as_str() == Some(id.as_str())) else {
            continue;
        };
        for dep in node["deps"].as_array().unwrap_or(&empty) {
            let built = dep["dep_kinds"]
                .as_array()
                .unwrap_or(&empty)
                .iter()
                .any(|kind| kind["kind"].is_null() || kind["kind"].as_str() == Some("build"));
            let Some(dep_id) = dep["pkg"].as_str() else {
                continue;
            };
            if built && local(dep_id).is_some() && seen.insert(dep_id.to_owned()) {
                queue.push(dep_id.to_owned());
            }
        }
    }
    let mut inputs = BTreeSet::new();
    for id in &seen {
        let Some(pkg) = local(id.as_str()) else {
            continue;
        };
        let Some(manifest) = pkg["manifest_path"].as_str().map(PathBuf::from) else {
            continue;
        };
        let dir = manifest.parent().map(Path::to_path_buf).unwrap_or_default();
        if dir.join("wit").is_dir() {
            inputs.insert(dir.join("wit"));
        }
        inputs.insert(manifest);
        for target in pkg["targets"].as_array().unwrap_or(&empty) {
            let kinds: Vec<&str> = target["kind"]
                .as_array()
                .unwrap_or(&empty)
                .iter()
                .filter_map(|k| k.as_str())
                .collect();
            let Some(src) = target["src_path"].as_str().map(PathBuf::from) else {
                continue;
            };
            if kinds.contains(&"custom-build") {
                inputs.insert(src);
            } else if !kinds
                .iter()
                .any(|k| matches!(*k, "test" | "bench" | "example"))
            {
                inputs.insert(src.parent().map(Path::to_path_buf).unwrap_or(src));
            }
        }
    }
    // A dir already covered by an ancestor dir is walked once.
    let all: Vec<PathBuf> = inputs.into_iter().collect();
    Ok(all
        .iter()
        .filter(|path| {
            !all.iter()
                .any(|other| other != *path && other.is_dir() && path.starts_with(other))
        })
        .cloned()
        .collect())
}

/// Parse a manifest written by [`write_manifest`]. Any other shape is an error
/// (delete the file and rebuild) — never silently an empty manifest.
pub fn parse_manifest(text: &str) -> Result<BTreeMap<String, Entry>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if value["schema"].as_u64() != Some(SCHEMA) {
        return Err(format!(
            "schema {} is not {SCHEMA} — delete it and run `{COMMAND}`",
            value["schema"]
        ));
    }
    let fixtures = value["fixtures"]
        .as_object()
        .ok_or("no `fixtures` object")?;
    fixtures
        .iter()
        .map(|(name, entry)| -> Result<(String, Entry), String> {
            let bad = || format!("entry `{name}` is not {{path, built_at, inputs}}");
            let path = entry["path"].as_str().ok_or_else(bad)?;
            let built_at = entry["built_at"].as_u64().ok_or_else(bad)?;
            let inputs = entry["inputs"]
                .as_array()
                .ok_or_else(bad)?
                .iter()
                .map(|input| input.as_str().map(PathBuf::from).ok_or_else(bad))
                .collect::<Result<_, _>>()?;
            Ok((
                name.clone(),
                Entry {
                    path: PathBuf::from(path),
                    built_at,
                    inputs,
                },
            ))
        })
        .collect()
}

/// Merge `entries` into the manifest at `path` (later builds add to earlier
/// ones, so CI jobs can build a subset with `--only`). One entry per line,
/// `"name": {"built_at":…,"inputs":[…],"path":"…"}` — the shape
/// `tests/harness/fixtures.rs` scans. Written to `manifest.json.tmp`, then
/// renamed over the manifest. An existing manifest that does not parse is an
/// error.
pub fn write_manifest(path: &Path, entries: &BTreeMap<String, Entry>) -> Result<(), String> {
    let mut merged = match std::fs::read_to_string(path) {
        Ok(text) => parse_manifest(&text).map_err(|e| format!("{}: {e}", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    for (name, entry) in entries {
        for text in std::iter::once(&entry.path)
            .chain(&entry.inputs)
            .map(|p| p.to_string_lossy())
        {
            if text.contains(['"', '\\']) {
                return Err(format!("fixture path {text} contains a quote or backslash"));
            }
        }
        merged.insert(name.clone(), entry.clone());
    }
    let lines: Vec<String> = merged
        .iter()
        .map(|(name, entry)| {
            let object = serde_json::json!({
                "path": entry.path,
                "built_at": entry.built_at,
                "inputs": entry.inputs,
            });
            format!("    {}: {object}", serde_json::Value::from(name.as_str()))
        })
        .collect();
    let body = format!(
        "{{\n  \"schema\": {SCHEMA},\n  \"command\": \"{COMMAND}\",\n  \"fixtures\": {{\n{}\n  }}\n}}\n",
        lines.join(",\n")
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{} -> {}: {e}", tmp.display(), path.display()))
}

fn unix_now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| format!("system clock before the unix epoch: {e}"))
}

pub fn run(release: bool, only: &[String], json: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("current dir: {e}"))?;
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let target = target_dir(&cargo, &root)?;
    let dir = match std::env::var_os("MAOS_FIXTURE_DIR").map(PathBuf::from) {
        Some(dir) if !dir.is_absolute() => {
            return Err(format!(
                "build-test-fixtures: MAOS_FIXTURE_DIR={} is not absolute (tests resolve it from \
                 their own cwd)",
                dir.display()
            ))
        }
        Some(dir) => dir,
        None => target.join("maos-fixtures"),
    };
    let table = fixtures(&root, &target, &dir, release);
    if let Some(unknown) = only
        .iter()
        .find(|wanted| !table.iter().any(|fixture| fixture.name == wanted.as_str()))
    {
        let known: Vec<&str> = table.iter().map(|fixture| fixture.name).collect();
        return Err(format!(
            "build-test-fixtures: unknown fixture `{unknown}` (known: {})",
            known.join(", ")
        ));
    }
    // Inputs common to every fixture: the workspace manifest (profiles) and the
    // root WIT contract guests and host bind (`../../wit/spirit.wit`).
    let shared: Vec<PathBuf> = [root.join("Cargo.toml"), root.join("wit")]
        .into_iter()
        .filter(|p| p.exists())
        .collect();
    let mut resolved: BTreeMap<PathBuf, serde_json::Value> = BTreeMap::new();
    let mut built = BTreeMap::new();
    for fixture in table
        .iter()
        .filter(|fixture| only.is_empty() || only.iter().any(|wanted| wanted == fixture.name))
    {
        if !resolved.contains_key(&fixture.manifest) {
            let manifest = path_arg(&fixture.manifest);
            let value = metadata(&cargo, &root, &["--locked", "--manifest-path", &manifest])?;
            resolved.insert(fixture.manifest.clone(), value);
        }
        let mut inputs = package_inputs(&resolved[&fixture.manifest], fixture.package)
            .map_err(|e| format!("build-test-fixtures: {}: {e}", fixture.name))?;
        inputs.extend(shared.iter().cloned());
        eprintln!(
            "build-test-fixtures: {} — cargo {}",
            fixture.name,
            fixture.args.join(" ")
        );
        let built_at = unix_now()?;
        let status = Command::new(&cargo)
            .args(&fixture.args)
            .envs(fixture.env.iter().copied())
            .current_dir(&root)
            .status()
            .map_err(|e| format!("cannot run cargo for {}: {e}", fixture.name))?;
        if !status.success() {
            return Err(format!(
                "build-test-fixtures: {} failed ({status})",
                fixture.name
            ));
        }
        if let Some((from, to)) = &fixture.copy {
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            std::fs::copy(from, to).map_err(|e| {
                format!(
                    "build-test-fixtures: {}: copy {} -> {}: {e}",
                    fixture.name,
                    from.display(),
                    to.display()
                )
            })?;
        }
        for (name, artefact) in &fixture.artefacts {
            if !artefact.is_file() {
                return Err(format!(
                    "build-test-fixtures: {} built but {} is missing",
                    fixture.name,
                    artefact.display()
                ));
            }
            built.insert(
                (*name).to_owned(),
                Entry {
                    path: artefact.clone(),
                    built_at,
                    inputs: inputs.clone(),
                },
            );
        }
    }
    let manifest = dir.join("manifest.json");
    write_manifest(&manifest, &built)?;
    let paths: BTreeMap<&String, &PathBuf> = built
        .iter()
        .map(|(name, entry)| (name, &entry.path))
        .collect();
    if json {
        println!(
            "{}",
            serde_json::json!({ "manifest": manifest, "built": paths })
        );
    } else {
        eprintln!(
            "build-test-fixtures: {} artefact(s) → {}",
            built.len(),
            manifest.display()
        );
    }
    Ok(())
}
