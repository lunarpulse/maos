#![allow(dead_code)]

//! Layout-agnostic location of the Cargo build output a test runs from.
//!
//! Cargo <= 1.99 puts a test executable at `<root>/<profile>/deps/<test>-<hash>`;
//! Cargo >= 1.100 (new build-dir layout) puts it at
//! `<root>/<profile>/build/<pkg>/<hash>/out/<test>-<hash>` and has no
//! `<profile>/deps` at all. Final artefacts (workspace binaries, `maos-fixtures/`)
//! sit in `<target>/<profile>/` and `<target>/` in both layouts, so a test that
//! reaches a sibling binary or the fixture manifest through
//! `current_exe().parent().parent()` silently resolves the wrong directory on
//! 1.100. Every test or harness that needs those locations asks this module.
//!
//! A same-package binary resolves through [`binary`]: the runtime
//! `NEXTEST_BIN_EXE_<name>` / `CARGO_BIN_EXE_<name>` (rebased by nextest when an
//! archive is extracted elsewhere), then `<profile>/<name>`, and only then the
//! compile-time `env!("CARGO_BIN_EXE_<name>")` the caller passes (an absolute
//! path of the machine that built the test, stale after a relocation).
//! A cross-package binary or fixture resolves, in order (first existing wins):
//!
//! 1. the directory of a runtime `NEXTEST_BIN_EXE_*` / `CARGO_BIN_EXE_*` path
//!    (nextest rebases the former for archive runs; both name `<profile>/<bin>`);
//! 2. an absolute `CARGO_TARGET_DIR` / `CARGO_BUILD_TARGET_DIR`: the profile
//!    directory the test executable sits under when it lies inside that target
//!    directory, else `<target>/<profile>` named by the executable (or the
//!    build's own `debug`/`release`);
//! 3. an ancestor walk from `current_exe()` that recognises both layouts.
//!
//! The fixture manifest directory is `$MAOS_FIXTURE_DIR` (published by the CI
//! job that ran the fixture builder; see `fixtures.rs`) or `<target>/maos-fixtures`.
//! When nothing exists the panic names every source it tried.
//!
//! Include with `#[path = "<rel>/tests/harness/layout.rs"] mod layout;` (or use
//! the re-exports of `fixtures.rs`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const BIN_EXE_PREFIXES: [&str; 2] = ["NEXTEST_BIN_EXE_", "CARGO_BIN_EXE_"];
const TARGET_DIR_VARS: [&str; 2] = ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"];

/// `<root>/<profile>` of a test executable in either Cargo layout, else `None`.
/// Pure path arithmetic; does not touch the filesystem.
pub(crate) fn walk_profile_dir(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    match dir.file_name()?.to_str()? {
        // `<profile>/deps/<test>-<hash>`
        "deps" => dir.parent().map(Path::to_path_buf),
        // `<profile>/build/<pkg>/<hash>/out/<test>-<hash>`
        "out" => {
            let build = dir.parent()?.parent()?.parent()?;
            if build.file_name()? == "build" {
                build.parent().map(Path::to_path_buf)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Absolute value of one environment variable of `vars`, or why it is unusable.
fn absolute_var<'a>(
    vars: &'a [(OsString, OsString)],
    name: &str,
) -> Option<Result<&'a Path, String>> {
    let (_, value) = vars.iter().find(|(key, _)| key == name)?;
    let path = Path::new(value);
    Some(if value.is_empty() {
        Err(format!("{name} is empty"))
    } else if path.is_absolute() {
        Ok(path)
    } else {
        // Cargo resolves it against the directory it was invoked from, which a
        // test cannot recover; guessing would hide a wrong directory.
        Err(format!("{name}={} is relative (ignored)", path.display()))
    })
}

/// The Cargo profile directory (`<target>/<profile>`) of the test executable
/// `exe`, given a snapshot of its environment. See the module docs for the order.
pub(crate) fn resolve_profile_dir(
    exe: &Path,
    vars: &[(OsString, OsString)],
) -> Result<PathBuf, String> {
    let walked = walk_profile_dir(exe);
    let mut candidates: Vec<(String, PathBuf)> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    let mut bin_vars: Vec<(&str, &Path)> = Vec::new();
    for prefix in BIN_EXE_PREFIXES {
        let mut named: Vec<(&str, &Path)> = vars
            .iter()
            .filter_map(|(key, value)| Some((key.to_str()?, Path::new(value))))
            .filter(|(key, _)| key.starts_with(prefix))
            .collect();
        named.sort();
        bin_vars.extend(named);
    }
    for (key, bin) in bin_vars {
        match bin.parent() {
            Some(dir) if bin.is_absolute() => {
                candidates.push((format!("{key}={}", bin.display()), dir.to_path_buf()))
            }
            _ => notes.push(format!(
                "{key}={} is not an absolute binary path",
                bin.display()
            )),
        }
    }

    let profile_name = walked
        .as_deref()
        .and_then(Path::file_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(if cfg!(debug_assertions) {
                "debug"
            } else {
                "release"
            })
        });
    for name in TARGET_DIR_VARS {
        match absolute_var(vars, name) {
            None => {}
            Some(Err(note)) => notes.push(note),
            Some(Ok(target)) => match &walked {
                Some(walked) if walked.starts_with(target) => candidates.push((
                    format!(
                        "{name}={} (test executable lies inside it)",
                        target.display()
                    ),
                    walked.clone(),
                )),
                _ => candidates.push((
                    format!("{name}={}/{}", target.display(), profile_name.display()),
                    target.join(&profile_name),
                )),
            },
        }
    }

    match &walked {
        Some(walked) => candidates.push((
            "ancestor walk of the test executable".to_owned(),
            walked.clone(),
        )),
        None => notes.push(
            "test executable is not under `<profile>/deps/` or \
             `<profile>/build/<pkg>/<hash>/out/`"
                .to_owned(),
        ),
    }

    if let Some((_, dir)) = candidates.iter().find(|(_, dir)| dir.is_dir()) {
        return Ok(dir.clone());
    }
    let tried: Vec<String> = candidates
        .iter()
        .map(|(source, dir)| format!("{source} -> {} (not a directory)", dir.display()))
        .chain(notes)
        .collect();
    Err(format!(
        "cannot locate the Cargo profile directory of test executable {}; tried: {}",
        exe.display(),
        if tried.is_empty() {
            "nothing".to_owned()
        } else {
            tried.join("; ")
        }
    ))
}

/// The Cargo target directory (`<target>`, where `maos-fixtures/` lives) for the
/// `profile` directory [`resolve_profile_dir`] returned: an explicit absolute
/// target directory the profile lies inside, else the profile's parent.
pub(crate) fn resolve_target_root(profile: &Path, vars: &[(OsString, OsString)]) -> PathBuf {
    TARGET_DIR_VARS
        .iter()
        .filter_map(|name| absolute_var(vars, name)?.ok())
        .find(|target| profile.starts_with(target))
        .map(Path::to_path_buf)
        .or_else(|| profile.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| profile.to_path_buf())
}

fn current() -> (PathBuf, Vec<(OsString, OsString)>) {
    (
        std::env::current_exe().expect("test executable path"),
        std::env::vars_os().collect(),
    )
}

/// `<target>/<profile>` of the running test: where workspace binaries such as
/// `maos`, `maosctl` and `worker-cli-fixture` are built. Layout-agnostic.
pub fn profile_dir() -> PathBuf {
    let (exe, vars) = current();
    resolve_profile_dir(&exe, &vars).unwrap_or_else(|error| panic!("{error}"))
}

/// `<target>` — the Cargo target directory the running test was built into.
pub fn target_root() -> PathBuf {
    let (_, vars) = current();
    resolve_target_root(&profile_dir(), &vars)
}

/// `PATH` with [`profile_dir`] prepended, so `maos` (and anything resolving a
/// worker CLI by name) finds the workspace binaries built for this profile.
pub fn path_with_profile() -> OsString {
    let mut paths = vec![profile_dir()];
    if let Some(existing) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(paths).expect("profile directory is a valid PATH entry")
}

/// The same-package binary `name` (`[[bin]]` of the test's own package), as
/// the running test must find it. `compile_time` is the caller's
/// `env!("CARGO_BIN_EXE_<name>")`. First hit wins: a runtime
/// `NEXTEST_BIN_EXE_<name>` / `CARGO_BIN_EXE_<name>` (absolute), then
/// `<profile>/<name>` of [`profile_dir`], then `compile_time`.
pub(crate) fn resolve_binary(
    name: &str,
    exe: &Path,
    vars: &[(OsString, OsString)],
    compile_time: &str,
) -> PathBuf {
    for prefix in BIN_EXE_PREFIXES {
        let key = format!("{prefix}{name}");
        let published = vars
            .iter()
            .find(|(candidate, _)| *candidate == *key)
            .map(|(_, value)| Path::new(value));
        if let Some(path) = published.filter(|path| path.is_absolute()) {
            return path.to_path_buf();
        }
    }
    if let Ok(profile) = resolve_profile_dir(exe, vars) {
        let beside = profile.join(name);
        if beside.is_file() {
            return beside;
        }
    }
    PathBuf::from(compile_time)
}

/// The same-package binary `name` of the running test; see [`resolve_binary`].
pub fn binary(name: &str, compile_time: &str) -> PathBuf {
    let (exe, vars) = current();
    resolve_binary(name, &exe, &vars, compile_time)
}

/// The profile directory named `profile_name` that sits beside `profile`
/// (`<target>[/<triple>]/<profile_name>`): the release runner of a debug test
/// keeps the `<triple>` of an explicit `--target` build.
pub(crate) fn sibling_profile_dir(profile: &Path, profile_name: &str) -> PathBuf {
    profile.with_file_name(profile_name)
}

/// Link `<exe_dir>/<name>` to `<profile>/<name>` when production's
/// daemon-sibling lookup (`maos_bin::worker_spawn::resolve_cli_binary`: the
/// executable's own directory, then its parent, then `$PATH`) could not
/// otherwise reach `profile`: a Cargo >= 1.100 test executable lives in
/// `<profile>/build/<pkg>/<hash>/out/`, four levels below it. Returns the link,
/// or `None` when the lookup already reaches `profile` (`exe_dir` is `profile`
/// or its `deps/` child). Atomic and idempotent across concurrent test processes.
pub(crate) fn stage_beside(
    exe_dir: &Path,
    profile: &Path,
    name: &str,
) -> std::io::Result<Option<PathBuf>> {
    if exe_dir == profile || exe_dir.parent() == Some(profile) {
        return Ok(None);
    }
    let target = profile.join(name);
    let link = exe_dir.join(name);
    if std::fs::read_link(&link).is_ok_and(|current| current == target) {
        return Ok(Some(link));
    }
    let scratch = exe_dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let _ = std::fs::remove_file(&scratch);
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &scratch)?;
    #[cfg(not(unix))]
    std::fs::copy(&target, &scratch)?;
    std::fs::rename(&scratch, &link)?;
    Ok(Some(link))
}

/// Make the `profile_dir()/<name>` binary reachable from production's
/// executable-relative lookup inside THIS test process (see [`stage_beside`]),
/// so an in-process caller of `resolve_cli_binary` finds it in either Cargo
/// layout without the production code knowing about test layouts. Returns
/// `profile_dir()/<name>`.
pub fn stage_beside_test_exe(name: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    let dir = exe.parent().expect("test executable directory");
    let profile = profile_dir();
    if let Err(error) = stage_beside(dir, &profile, name) {
        panic!(
            "cannot stage {name} beside the test executable {}: {error}",
            exe.display()
        );
    }
    profile.join(name)
}
