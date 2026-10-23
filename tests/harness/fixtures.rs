#![allow(dead_code)]

//! Story 20-12 AC2 — prebuilt test fixtures. No test compiles the tree: every
//! binary or WASM artefact a test needs is built ONCE by
//! `cargo run -p xtask -- build-test-fixtures` (a CI step before the suite) and
//! resolved here. A missing fixture fails with that command in the message.
//!
//! A STALE fixture fails the same way (review 2026-10-23): the manifest records
//! per entry `built_at` and the `inputs` its build read (source roots of the
//! fixture's local dependency closure, manifests, WIT); any `.rs`/`.toml`/`.wit`
//! input newer than the artefact means the artefact is not the code under test.
//!
//! Include with `#[path = "<rel>/tests/harness/fixtures.rs"] mod fixtures;`.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The one command that builds every fixture resolved here.
pub const BUILD_FIXTURES: &str = "cargo run -p xtask -- build-test-fixtures";

/// `<target>/<profile>` of the running test binary
/// (`<target>/<profile>/deps/<test>-<hash>`).
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    exe.parent()
        .and_then(Path::parent)
        .expect("test binary lives in <target>/<profile>/deps")
        .to_path_buf()
}

/// `<target>` — the Cargo target directory the running test was built into.
pub fn target_root() -> PathBuf {
    profile_dir()
        .parent()
        .expect("<target>/<profile> has a parent")
        .to_path_buf()
}

/// `$MAOS_FIXTURE_DIR` (absolute — the builder refuses a relative one too),
/// else `<target>/maos-fixtures` (where the builder writes `manifest.json`).
pub fn fixture_dir() -> PathBuf {
    match std::env::var_os("MAOS_FIXTURE_DIR").map(PathBuf::from) {
        Some(dir) if !dir.is_absolute() => panic!(
            "MAOS_FIXTURE_DIR={} is not absolute; set an absolute path for both `{BUILD_FIXTURES}` \
             and the test run",
            dir.display()
        ),
        Some(dir) => dir,
        None => target_root().join("maos-fixtures"),
    }
}

fn missing(what: &str) -> ! {
    panic!("test fixture missing — {what}; build fixtures first: `{BUILD_FIXTURES}`")
}

fn stale(what: &str) -> ! {
    panic!("test fixture stale — {what}; rebuild fixtures: `{BUILD_FIXTURES}`")
}

/// One manifest entry: the artefact, its build start and the inputs it read.
struct Entry {
    path: PathBuf,
    built_at: u64,
    inputs: Vec<PathBuf>,
}

/// The manifest entry for `name`. The builder writes one
/// `"name": {"built_at":N,"inputs":["…"],"path":"…"}` entry per line and refuses
/// paths containing `"` or `\`, so a substring scan is exact. `Ok(None)`: no
/// manifest, or no entry for `name`; `Err`: unreadable or malformed.
fn manifest_entry(name: &str) -> Result<Option<Entry>, String> {
    let manifest = fixture_dir().join("manifest.json");
    let text = match std::fs::read_to_string(&manifest) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", manifest.display())),
    };
    let key = format!("\"{name}\": {{");
    let Some(line) = text.lines().find_map(|line| line.trim().strip_prefix(&key)) else {
        return Ok(None);
    };
    let malformed = || format!("malformed `{name}` entry in {}", manifest.display());
    let field = |key: &str| {
        line.split_once(&format!("\"{key}\":"))
            .map(|(_, rest)| rest)
            .ok_or_else(malformed)
    };
    let path = field("path")?
        .strip_prefix('"')
        .and_then(|rest| rest.split('"').next())
        .ok_or_else(malformed)?;
    let built_at = field("built_at")?
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .and_then(|digits| digits.parse().ok())
        .ok_or_else(malformed)?;
    // `["a","b"]` split on `"`: odd parts are the strings; an even part holding
    // `]` closes the array.
    let mut inputs = Vec::new();
    let array = field("inputs")?.strip_prefix('[').ok_or_else(malformed)?;
    for (index, part) in array.split('"').enumerate() {
        if index % 2 == 1 {
            inputs.push(PathBuf::from(part));
        } else if part.contains(']') {
            break;
        }
    }
    Ok(Some(Entry {
        path: PathBuf::from(path),
        built_at,
        inputs,
    }))
}

/// The first `.rs`/`.toml`/`.wit` file under `inputs` (files or dirs; `target/`,
/// hidden dirs and symlinks skipped) modified after `reference`.
fn newer_input(inputs: &[PathBuf], reference: SystemTime) -> Option<PathBuf> {
    let mut queue: Vec<PathBuf> = inputs.to_vec();
    while let Some(path) = queue.pop() {
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            let skip = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "target" || name.starts_with('.'));
            if !skip {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    queue.extend(entries.filter_map(Result::ok).map(|entry| entry.path()));
                }
            }
        } else if meta.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "rs" || ext == "toml" || ext == "wit")
            && meta.modified().is_ok_and(|modified| modified > reference)
        {
            return Some(path);
        }
    }
    None
}

fn modified(path: &Path) -> SystemTime {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .unwrap_or_else(|error| missing(&format!("cannot stat {}: {error}", path.display())))
}

/// The artefact the fixture manifest names `name` (e.g. `echo_spirit.wasm`,
/// `maos-wasm-runner`, `forbidden-syscall-probe`). Panics naming
/// [`BUILD_FIXTURES`] when the manifest, the entry or the file is absent, or
/// when an input is newer than the entry's `built_at`.
pub fn fixture(name: &str) -> PathBuf {
    let entry = match manifest_entry(name) {
        Ok(Some(entry)) => entry,
        Ok(None) => missing(&format!(
            "no `{name}` entry in {} (or no manifest)",
            fixture_dir().join("manifest.json").display()
        )),
        Err(error) => missing(&error),
    };
    if !entry.path.is_file() {
        missing(&format!(
            "`{name}` is listed but {} does not exist",
            entry.path.display()
        ));
    }
    let built = UNIX_EPOCH + Duration::from_secs(entry.built_at);
    if let Some(input) = newer_input(&entry.inputs, built) {
        stale(&format!(
            "`{name}` ({}) is older than its input {}",
            entry.path.display(),
            input.display()
        ));
    }
    entry.path
}

/// A workspace binary that must sit next to the running test's profile
/// (`<target>/<profile>/<name>` — e.g. `worker-cli-fixture`, which `maos`
/// resolves beside its own executable, or `maosctl`). Panics naming
/// [`BUILD_FIXTURES`] (with `--release` for a release-profile test run) when it
/// is absent, or when a manifest entry for `name` records an input newer than
/// this file (the profile's own binary, so its mtime is the reference). A
/// binary the builder did not record (no manifest, or no entry — built by a CI
/// step's own `cargo build`) is checked for presence only.
pub fn profile_binary(name: &str) -> PathBuf {
    let path = profile_dir().join(name);
    let release = if cfg!(debug_assertions) {
        ""
    } else {
        " --release"
    };
    if !path.is_file() {
        panic!(
            "test fixture missing — {} does not exist; build fixtures first: `{BUILD_FIXTURES}{release}`",
            path.display()
        );
    }
    let inputs = match manifest_entry(name) {
        Ok(Some(entry)) => entry.inputs,
        Ok(None) => Vec::new(),
        Err(error) => panic!(
            "test fixture manifest — {error}; build fixtures first: `{BUILD_FIXTURES}{release}`"
        ),
    };
    if let Some(input) = newer_input(&inputs, modified(&path)) {
        panic!(
            "test fixture stale — {} is older than its input {}; rebuild fixtures: \
             `{BUILD_FIXTURES}{release}`",
            path.display(),
            input.display()
        );
    }
    path
}
