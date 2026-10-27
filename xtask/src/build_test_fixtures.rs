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
//! **Staleness.** `MAOS_FIXTURE_TRANSPORT=1` requires a clean checkout and records
//! its Git tree and each artefact's Git blob identity in schema 3. The harness
//! verifies both identities without trusting checkout mtimes. Ordinary own-build
//! runs intentionally retain the schema-2 `built_at`/`inputs` mtime contract.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The manifest format the harness parses (`tests/harness/fixtures.rs`).
const SCHEMA: u64 = 3;
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

/// One manifest entry. Unknown (e.g. renamed provenance) fields are refused,
/// never dropped by a merge that would rewrite a sealed entry as unsealed.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: PathBuf,
    pub built_at: u64,
    pub inputs: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact: Option<String>,
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
            let kinds = target["kind"].as_array().unwrap_or(&empty);
            let has_kind = |kind| kinds.iter().any(|value| value.as_str() == Some(kind));
            let Some(src) = target["src_path"].as_str().map(PathBuf::from) else {
                continue;
            };
            if has_kind("custom-build") {
                inputs.insert(src);
            } else if !["test", "bench", "example"].into_iter().any(has_kind) {
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
    if !matches!(value["schema"].as_u64(), Some(2 | SCHEMA)) {
        return Err(format!(
            "schema {} is not {SCHEMA} — delete it and run `{COMMAND}`",
            value["schema"]
        ));
    }
    let fixtures: BTreeMap<String, Entry> = serde_json::from_value(value["fixtures"].clone())
        .map_err(|e| format!("invalid fixtures: {e}"))?;
    for (name, entry) in &fixtures {
        if entry.tree.is_some() != entry.artifact.is_some()
            || (value["schema"].as_u64() == Some(2) && entry.tree.is_some())
            || [&entry.tree, &entry.artifact]
                .into_iter()
                .flatten()
                .any(|id| {
                    !matches!(id.len(), 40 | 64) || !id.bytes().all(|c| c.is_ascii_hexdigit())
                })
        {
            return Err(format!(
                "entry `{name}` needs valid schema-3 tree and artifact identities"
            ));
        }
    }
    Ok(fixtures)
}

/// Merge entries, accepting schema 2's local mtime entries and writing schema
/// 3. One compact entry per line is also parsed by the dependency-free harness.
/// An unreadable or malformed existing manifest is never silently discarded.
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
            if text.bytes().any(|b| b == b'"' || b == b'\\' || b < 0x20) {
                return Err(format!("fixture path {text:?} has quote/backslash/control"));
            }
        }
        merged.insert(name.clone(), entry.clone());
    }
    let lines: Vec<String> = merged
        .iter()
        .map(|(name, entry)| {
            serde_json::to_string(entry)
                .map(|value| format!("    {}: {value}", serde_json::Value::from(name.as_str())))
                .map_err(|error| format!("cannot serialize fixture `{name}`: {error}"))
        })
        .collect::<Result<_, _>>()?;
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

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("fixture provenance: git {args:?}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "fixture provenance: git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// Own-build mode skips Git; explicit transport refuses dirty source trees.
pub fn clean_tree(root: &Path, transport: bool) -> Result<Option<String>, String> {
    if !transport {
        return Ok(None);
    }
    let status = git(
        root,
        &["status", "--porcelain", "-uall", "--ignore-submodules=none"],
    )?;
    if !status.is_empty() {
        return Err("fixture transport requires a clean checkout; source tree is dirty".into());
    }
    git(root, &["rev-parse", "HEAD^{tree}"]).map(Some)
}

/// Seal only a build whose initially clean source tree stayed unchanged.
pub fn artifact(root: &Path, path: &Path, tree: &Option<String>) -> Result<Option<String>, String> {
    let Some(expected) = tree else {
        return Ok(None);
    };
    if clean_tree(root, true)?.as_ref() != Some(expected) {
        return Err(
            "fixture provenance: checkout changed during the build; rebuild fixtures".into(),
        );
    }
    git(
        root,
        &["hash-object", "--no-filters", "--", &path_arg(path)],
    )
    .map(Some)
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
    let transport = std::env::var("MAOS_FIXTURE_TRANSPORT").as_deref() == Ok("1");
    let tree = clean_tree(&root, transport)?;
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
                    artifact: artifact(&root, artefact, &tree)?,
                    tree: tree.clone(),
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
