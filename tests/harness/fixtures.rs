#![allow(dead_code)]

//! Story 20-12 AC2 — prebuilt test fixtures. No test compiles the tree: every
//! binary or WASM artefact a test needs is built ONCE by
//! `cargo run -p xtask -- build-test-fixtures` (a CI step before the suite) and
//! resolved here. A missing fixture fails with that command in the message.
//!
//! Schema 3 entries may record a clean Git `tree` and `artifact` blob identity:
//! a clean matching checkout accepts those bytes regardless of checkout mtimes.
//! Dirty/mismatched checkouts and changed artefacts fail without rebuilding.
//! `MAOS_FIXTURE_TRANSPORT=1` requires both identities. Ordinary own-build runs,
//! including CI, retain schema 2/local schema-3 `built_at`/`inputs` mtime checks.
//! These checks refuse stale/corrupt transport, not maliciously recomputed IDs;
//! the manifest is not an authenticated signature or a producer attestation.
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
pub(crate) struct Entry {
    path: PathBuf,
    built_at: u64,
    inputs: Vec<PathBuf>,
    tree: Option<String>,
    artifact: Option<String>,
}

/// The builder writes one compact entry per line. Validate the entire generated
/// envelope and every entry before using any identity, without a JSON dependency.
fn manifest_entry(name: &str) -> Result<Option<Entry>, String> {
    let manifest = fixture_dir().join("manifest.json");
    let text = match std::fs::read_to_string(&manifest) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("cannot read {}: {error}", manifest.display())),
    };
    parse_manifest_entry(&text, name, &manifest)
}

pub(crate) fn parse_manifest_entry(
    text: &str,
    name: &str,
    manifest: &Path,
) -> Result<Option<Entry>, String> {
    let malformed = || format!("malformed fixture manifest in {}", manifest.display());
    let mut lines = text.lines().map(str::trim);
    if lines.next() != Some("{") {
        return Err(malformed());
    }
    let schema = lines
        .next()
        .and_then(|line| line.strip_prefix("\"schema\":"))
        .and_then(|value| value.trim().strip_suffix(','))
        .and_then(|value| match value.trim() {
            "2" => Some(2),
            "3" => Some(3),
            _ => None,
        })
        .ok_or_else(malformed)?;
    let mut header = lines.next();
    if header
        .and_then(|line| line.strip_prefix("\"command\": \""))
        .and_then(|value| value.strip_suffix("\","))
        == Some(BUILD_FIXTURES)
    {
        header = lines.next();
    }
    if header != Some("\"fixtures\": {") {
        return Err(malformed());
    }
    let mut found = None;
    let mut names = std::collections::BTreeSet::new();
    let mut comma = true;
    let mut any = false;
    loop {
        let line = lines.next().ok_or_else(malformed)?;
        if line == "}" {
            if any && comma {
                return Err(malformed());
            }
            break;
        }
        // An empty fixture map has one blank body line in the generated format.
        if line.is_empty() && !any {
            continue;
        }
        if !comma {
            return Err(malformed());
        }
        comma = line.ends_with(',');
        let mut row = ManifestRow(line.strip_suffix(',').unwrap_or(line));
        let parsed = (|| {
            let key = row.string()?;
            if !names.insert(key) {
                return None;
            }
            row.take(":")?;
            let entry = row.entry(schema)?;
            if !row.0.is_empty() {
                return None;
            }
            Some((key, entry))
        })()
        .ok_or_else(malformed)?;
        if parsed.0 == name {
            found = Some(parsed.1);
        }
        any = true;
    }
    if lines.next() != Some("}") || lines.any(|line| !line.is_empty()) {
        return Err(malformed());
    }
    Ok(found)
}

/// A deliberately small grammar for the writer's unescaped path/string format,
/// not a general JSON parser. Unknown/duplicate fields and incomplete JSON fail.
struct ManifestRow<'a>(&'a str);

impl<'a> ManifestRow<'a> {
    fn take(&mut self, token: &str) -> Option<()> {
        self.0 = self.0.trim_start().strip_prefix(token)?.trim_start();
        Some(())
    }

    fn string(&mut self) -> Option<&'a str> {
        self.0 = self.0.trim_start().strip_prefix('"')?;
        let (value, rest) = self.0.split_once('"')?;
        if value.bytes().any(|c| c == b'\\' || c < 0x20) {
            return None;
        }
        self.0 = rest.trim_start();
        Some(value)
    }

    fn entry(&mut self, schema: u64) -> Option<Entry> {
        self.take("{")?;
        let (mut path, mut built_at, mut inputs, mut tree, mut artifact) =
            (None, None, None, None, None);
        loop {
            let key = self.string()?;
            self.take(":")?;
            match key {
                "path" if path.is_none() => path = Some(PathBuf::from(self.string()?)),
                "built_at" if built_at.is_none() => {
                    let end = self
                        .0
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(self.0.len());
                    let digits = &self.0[..end];
                    if digits.len() > 1 && digits.starts_with('0') {
                        return None;
                    }
                    built_at = Some(digits.parse().ok()?);
                    self.0 = self.0[end..].trim_start();
                }
                "inputs" if inputs.is_none() => {
                    self.take("[")?;
                    let mut paths = Vec::new();
                    if !self.0.starts_with(']') {
                        loop {
                            paths.push(PathBuf::from(self.string()?));
                            if self.0.starts_with(']') {
                                break;
                            }
                            self.take(",")?;
                        }
                    }
                    self.take("]")?;
                    inputs = Some(paths);
                }
                "tree" | "artifact" => {
                    let slot = if key == "tree" {
                        &mut tree
                    } else {
                        &mut artifact
                    };
                    if slot.is_some() || schema != 3 {
                        return None;
                    }
                    let value = self.string()?;
                    if !matches!(value.len(), 40 | 64)
                        || !value.bytes().all(|c| c.is_ascii_hexdigit())
                    {
                        return None;
                    }
                    *slot = Some(value.to_owned());
                }
                _ => return None,
            }
            if self.0.starts_with('}') {
                self.take("}")?;
                break;
            }
            self.take(",")?;
        }
        if tree.is_some() != artifact.is_some() {
            return None;
        }
        Some(Entry {
            path: path?,
            built_at: built_at?,
            inputs: inputs?,
            tree,
            artifact,
        })
    }
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

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    git_stdout(root, args).map(|stdout| stdout.trim().to_owned())
}

/// Repository-locating variables a hook or tool may export would redirect
/// these queries to another repository; `HOME`/config (`safe.directory`) stay.
const GIT_LOCATORS: [&str; 8] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
];

/// Untrimmed stdout: a porcelain entry's leading status column is significant.
fn git_stdout(root: &Path, args: &[&str]) -> Result<String, String> {
    let mut command = std::process::Command::new("git");
    for locator in GIT_LOCATORS {
        command.env_remove(locator);
    }
    let output = command
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("cannot verify fixture provenance: git {args:?}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cannot verify fixture provenance: git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Entries a dirty-checkout refusal names; the rest are only counted.
const DIRTY_ENTRIES_SHOWN: usize = 8;

/// Names the offending `git status --porcelain` entries (bounded) so a hosted
/// refusal identifies what dirtied the checkout without a rerun.
fn dirty_checkout(status: &str) -> String {
    let entries: Vec<&str> = status.lines().collect();
    let shown = entries.len().min(DIRTY_ENTRIES_SHOWN);
    let more = match entries.len() - shown {
        0 => String::new(),
        hidden => format!(" | … {hidden} more"),
    };
    format!(
        "checkout is dirty; clean provenance cannot authorize changed files; \
         git status --porcelain: {}{more}",
        entries[..shown].join(" | ")
    )
}

/// Timestamp compatibility for own-built bytes never authorizes explicit transport.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationMode {
    Local,
    Transport,
}

impl VerificationMode {
    pub(crate) fn from_transport(transport: bool) -> Self {
        if transport {
            Self::Transport
        } else {
            Self::Local
        }
    }

    fn runtime() -> Self {
        Self::from_transport(std::env::var("MAOS_FIXTURE_TRANSPORT").as_deref() == Ok("1"))
    }
}

/// Provenance is all-or-nothing: never fall back to timestamps after a failed
/// tree/cleanliness/blob check. The profile lookup must also name these bytes.
pub(crate) fn verify_entry(
    entry: &Entry,
    path: &Path,
    reference: SystemTime,
    root: &Path,
    mode: VerificationMode,
) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("{} does not exist", path.display()));
    }
    if mode == VerificationMode::Transport && (entry.tree.is_none() || entry.artifact.is_none()) {
        return Err("fixture transport requires clean tree and artifact provenance".to_owned());
    }
    if let Some(tree) = &entry.tree {
        let listed = std::fs::canonicalize(&entry.path)
            .map_err(|error| format!("cannot resolve {}: {error}", entry.path.display()))?;
        let actual = std::fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve {}: {error}", path.display()))?;
        if listed != actual {
            return Err(format!(
                "manifest names {}, not profile binary {}",
                listed.display(),
                actual.display()
            ));
        }
        let status = git_stdout(
            root,
            &[
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )?;
        let status = status.trim_end();
        if !status.is_empty() {
            return Err(dirty_checkout(status));
        }
        let checkout = git(root, &["rev-parse", "HEAD^{tree}"])?;
        if tree != &checkout {
            return Err(format!(
                "manifest tree {tree} differs from checkout tree {checkout}"
            ));
        }
        let artifact = git(
            root,
            &[
                "hash-object",
                "--no-filters",
                "--",
                path.to_str().ok_or("fixture path is not UTF-8")?,
            ],
        )?;
        if entry.artifact.as_ref() != Some(&artifact) {
            return Err(format!("artifact identity mismatch for {}", path.display()));
        }
    } else if let Some(input) = newer_input(&entry.inputs, reference) {
        return Err(format!(
            "{} is older than its input {}",
            path.display(),
            input.display()
        ));
    }
    Ok(())
}

/// Profile-coupled fixtures belong to the manifest; product binaries such as
/// `maos` may instead be supplied by the CI job's separate feature-family build.
pub(crate) fn verify_profile_entry(
    entry: Option<&Entry>,
    name: &str,
    path: &Path,
    reference: SystemTime,
    root: &Path,
    mode: VerificationMode,
) -> Result<(), String> {
    match entry {
        Some(entry) => verify_entry(entry, path, reference, root, mode),
        None if mode == VerificationMode::Transport
            && matches!(
                name,
                "maosctl" | "worker-cli-fixture" | "worker-probe-fixture"
            ) =>
        {
            Err(format!(
                "no provenance entry for manifest-owned profile binary `{name}`"
            ))
        }
        None if !path.is_file() => Err(format!("{} does not exist", path.display())),
        None => Ok(()),
    }
}

/// The artefact the fixture manifest names `name` (e.g. `echo_spirit.wasm`,
/// `maos-wasm-runner`, `forbidden-syscall-probe`). Panics naming
/// [`BUILD_FIXTURES`] when absent, when provenance fails, or (for local entries)
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
    // A listed but absent artefact is unbuilt, not stale provenance.
    if !entry.path.is_file() {
        missing(&format!(
            "`{name}`: {} does not exist",
            entry.path.display()
        ));
    }
    let root = std::env::current_dir().expect("fixture checkout directory");
    let built = UNIX_EPOCH + Duration::from_secs(entry.built_at);
    if let Err(error) = verify_entry(
        &entry,
        &entry.path,
        built,
        &root,
        VerificationMode::runtime(),
    ) {
        stale(&format!("`{name}`: {error}"));
    }
    entry.path
}

/// A workspace binary that must sit next to the running test's profile
/// (`<target>/<profile>/<name>` — e.g. `worker-cli-fixture`, which `maos`
/// resolves beside its own executable, or `maosctl`). Panics naming
/// [`BUILD_FIXTURES`] (with `--release` for a release-profile test run) when
/// absent or provenance fails. Own-build runs retain the profile binary's mtime
/// as reference for entries without a tree. Explicit fixture transport requires
/// manifest entries for profile-coupled fixtures, but separately built, unrecorded
/// product binaries retain their presence-only check.
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
    let entry = match manifest_entry(name) {
        Ok(entry) => entry,
        Err(error) => panic!(
            "test fixture manifest — {error}; build fixtures first: `{BUILD_FIXTURES}{release}`"
        ),
    };
    let root = std::env::current_dir().expect("fixture checkout directory");
    if let Err(error) = verify_profile_entry(
        entry.as_ref(),
        name,
        &path,
        modified(&path),
        &root,
        VerificationMode::runtime(),
    ) {
        panic!("test fixture stale — {error}; rebuild fixtures: `{BUILD_FIXTURES}{release}`");
    }
    path
}
