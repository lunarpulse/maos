//! `tests/harness/layout.rs` — the shared, layout-agnostic locator every test
//! uses for sibling binaries and the fixture manifest. Pure path arithmetic plus
//! real temporary directories; no compiler, no network.
//!
//! The two Cargo layouts of a test executable under `<target>/<profile>/`:
//! `deps/<test>-<hash>` (<= 1.99) and `build/<pkg>/<hash>/out/<test>-<hash>`
//! (>= 1.100).

#[path = "../../tests/harness/layout.rs"]
mod layout;

use std::ffi::OsString;
use std::path::{Path, PathBuf};

const LEGACY: &str = "deps/probe-0123456789abcdef";
const BUILD_DIR: &str = "build/maos-bin/0123456789abcdef/out/probe-0123456789abcdef";

fn vars(pairs: &[(&str, &Path)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(key, value)| (OsString::from(key), value.as_os_str().to_owned()))
        .collect()
}

/// A target dir holding `<target>/<profile>/` and the test executable's dir.
fn target(profile: &str, exe: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let profile_dir = dir.path().join("target").join(profile);
    let exe = profile_dir.join(exe);
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    (dir, profile_dir, exe)
}

#[test]
fn walk_recognises_both_cargo_layouts() {
    for (exe, layout_name) in [(LEGACY, "deps"), (BUILD_DIR, "build/<pkg>/<hash>/out")] {
        let (_dir, profile, exe) = target("debug", exe);
        assert_eq!(
            layout::walk_profile_dir(&exe),
            Some(profile),
            "{layout_name} layout"
        );
    }
}

#[test]
fn walk_refuses_directories_that_are_not_a_cargo_layout() {
    for exe in [
        "/tmp/probe",
        "/tmp/out/probe",
        "/tmp/a/b/out/probe",
        "/tmp/profile/notbuild/pkg/hash/out/probe",
        "/tmp/profile/build/hash/out/probe",
    ] {
        assert_eq!(layout::walk_profile_dir(Path::new(exe)), None, "{exe}");
    }
}

#[test]
fn ancestor_walk_resolves_both_layouts_without_any_environment() {
    for exe in [LEGACY, BUILD_DIR] {
        let (_dir, profile, exe) = target("release", exe);
        assert_eq!(layout::resolve_profile_dir(&exe, &[]).unwrap(), profile);
        assert_eq!(
            layout::resolve_target_root(&profile, &[]),
            profile.parent().unwrap()
        );
    }
}

#[test]
fn a_published_binary_path_wins_over_the_executable_location() {
    // nextest archive / separate build-dir: the executable sits somewhere that
    // is not the target dir; the bin path names the real profile directory.
    let (_build, _build_profile, exe) = target("debug", BUILD_DIR);
    let (_real, real_profile, _) = target("debug", LEGACY);
    let bin = real_profile.join("maos");
    for name in ["NEXTEST_BIN_EXE_maos", "CARGO_BIN_EXE_maos"] {
        assert_eq!(
            layout::resolve_profile_dir(&exe, &vars(&[(name, &bin)])).unwrap(),
            real_profile,
            "{name}"
        );
    }
}

#[test]
fn an_unusable_published_binary_path_falls_through_to_the_next_source() {
    let (_dir, profile, exe) = target("debug", BUILD_DIR);
    let gone = Path::new("/nonexistent-layout-probe/debug/maos");
    assert_eq!(
        layout::resolve_profile_dir(&exe, &vars(&[("CARGO_BIN_EXE_maos", gone)])).unwrap(),
        profile
    );
    assert_eq!(
        layout::resolve_profile_dir(&exe, &vars(&[("CARGO_BIN_EXE_maos", Path::new("maos"))]))
            .unwrap(),
        profile,
        "a relative binary path is ignored"
    );
}

#[test]
fn an_explicit_target_dir_outside_the_executable_names_the_profile() {
    // Build-dir split from target-dir: the executable is under `<build-dir>`,
    // the final artefacts under `<target>`.
    let (_build, _profile, exe) = target("debug", BUILD_DIR);
    let (real, real_profile, _) = target("debug", LEGACY);
    let target_dir = real.path().join("target");
    for name in ["CARGO_TARGET_DIR", "CARGO_BUILD_TARGET_DIR"] {
        let resolved = layout::resolve_profile_dir(&exe, &vars(&[(name, &target_dir)])).unwrap();
        assert_eq!(resolved, real_profile, "{name}");
        assert_eq!(
            layout::resolve_target_root(&resolved, &vars(&[(name, &target_dir)])),
            target_dir,
            "{name}"
        );
    }
}

#[test]
fn an_explicit_target_dir_containing_the_executable_keeps_cross_target_profiles() {
    // `--target <triple>` puts the profile at `<target>/<triple>/<profile>`.
    let dir = tempfile::tempdir().unwrap();
    let target_dir = dir.path().join("target");
    let profile = target_dir.join("x86_64-unknown-linux-gnu/debug");
    let exe = profile.join(BUILD_DIR);
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::create_dir_all(target_dir.join("debug")).unwrap();
    let env = vars(&[("CARGO_TARGET_DIR", &target_dir)]);
    let resolved = layout::resolve_profile_dir(&exe, &env).unwrap();
    assert_eq!(resolved, profile);
    assert_eq!(layout::resolve_target_root(&resolved, &env), target_dir);
}

#[test]
fn a_relative_target_dir_is_ignored_not_guessed() {
    let (_dir, profile, exe) = target("debug", LEGACY);
    let env = vars(&[("CARGO_TARGET_DIR", Path::new("target"))]);
    assert_eq!(layout::resolve_profile_dir(&exe, &env).unwrap(), profile);
}

#[test]
fn failure_names_every_source_that_was_tried() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("somewhere/probe");
    let missing = dir.path().join("missing-target");
    let bin = dir.path().join("missing-profile/maos");
    let error = layout::resolve_profile_dir(
        &exe,
        &vars(&[("CARGO_TARGET_DIR", &missing), ("CARGO_BIN_EXE_maos", &bin)]),
    )
    .unwrap_err();
    for expected in [
        "cannot locate the Cargo profile directory",
        "CARGO_BIN_EXE_maos=",
        "CARGO_TARGET_DIR=",
        "not under `<profile>/deps/` or `<profile>/build/<pkg>/<hash>/out/`",
    ] {
        assert!(error.contains(expected), "{expected} not in: {error}");
    }
}

/// The running test executable resolves to a directory that really holds this
/// toolchain's build output, in whichever layout it was built with.
#[test]
fn the_running_test_resolves_its_own_profile_directory() {
    let exe = std::env::current_exe().unwrap();
    let profile = layout::profile_dir();
    assert!(profile.is_dir(), "{}", profile.display());
    assert!(
        exe.starts_with(&profile) || std::env::var_os("CARGO_TARGET_DIR").is_some(),
        "{} is not under {}",
        exe.display(),
        profile.display()
    );
    assert!(layout::target_root().is_dir());
    assert!(profile.starts_with(layout::target_root()));
    assert_eq!(
        std::env::split_paths(&layout::path_with_profile()).next(),
        Some(profile)
    );
}

#[test]
fn the_release_sibling_keeps_the_target_triple_of_a_cross_target_build() {
    let dir = tempfile::tempdir().unwrap();
    let target_dir = dir.path().join("target");
    let profile = target_dir.join("x86_64-unknown-linux-gnu/debug");
    let exe = profile.join(BUILD_DIR);
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    let env = vars(&[("CARGO_TARGET_DIR", &target_dir)]);
    let resolved = layout::resolve_profile_dir(&exe, &env).unwrap();
    assert_eq!(
        layout::sibling_profile_dir(&resolved, "release"),
        target_dir.join("x86_64-unknown-linux-gnu/release"),
        "the release runner sits beside the resolved profile directory"
    );
    assert_ne!(
        layout::sibling_profile_dir(&resolved, "release"),
        layout::resolve_target_root(&resolved, &env).join("release"),
        "the target root drops `<triple>`"
    );
    let (_plain, plain_profile, _) = target("debug", LEGACY);
    assert_eq!(
        layout::sibling_profile_dir(&plain_profile, "release"),
        plain_profile.with_file_name("release")
    );
}

#[test]
fn a_same_package_binary_prefers_runtime_relocation_over_the_compile_time_path() {
    let (_dir, profile, exe) = target("debug", BUILD_DIR);
    std::fs::write(profile.join("runner"), b"beside").unwrap();
    let (_moved, moved_profile, _) = target("debug", LEGACY);
    let rebased = moved_profile.join("runner");
    let compile_time = "/producer/target/debug/runner";

    for name in ["NEXTEST_BIN_EXE_runner", "CARGO_BIN_EXE_runner"] {
        assert_eq!(
            layout::resolve_binary("runner", &exe, &vars(&[(name, &rebased)]), compile_time),
            rebased,
            "{name}: the rebased runtime path wins, even over `<profile>/runner`"
        );
    }
    assert_eq!(
        layout::resolve_binary("runner", &exe, &[], compile_time),
        profile.join("runner"),
        "no published path: the layout-aware `<profile>/runner`"
    );
    assert_eq!(
        layout::resolve_binary(
            "runner",
            &exe,
            &vars(&[("CARGO_BIN_EXE_runner", Path::new("runner"))]),
            compile_time
        ),
        profile.join("runner"),
        "a relative published path is ignored"
    );
    std::fs::remove_file(profile.join("runner")).unwrap();
    assert_eq!(
        layout::resolve_binary("runner", &exe, &[], compile_time),
        PathBuf::from(compile_time),
        "only then the compile-time path"
    );
}

/// Production's `resolve_cli_binary` checks `<exe dir>/<cmd>` then
/// `<exe dir>/../<cmd>`; the harness must make that lookup succeed in both
/// Cargo layouts without production knowing about either.
fn production_lookup(exe: &Path, command: &str) -> Option<PathBuf> {
    let dir = exe.parent()?;
    [Some(dir), dir.parent()]
        .into_iter()
        .flatten()
        .map(|base| base.join(command))
        .find(|candidate| candidate.is_file())
}

#[test]
fn staging_makes_the_production_sibling_lookup_reach_the_fixture_in_both_layouts() {
    for (exe_layout, links) in [(LEGACY, false), (BUILD_DIR, true)] {
        let (_dir, profile, exe) = target("debug", exe_layout);
        let fixture = profile.join("fixture-cli");
        std::fs::write(&fixture, b"fixture").unwrap();
        let exe_dir = exe.parent().unwrap();
        let found_before = production_lookup(&exe, "fixture-cli");
        assert_eq!(
            found_before.is_some(),
            !links,
            "{exe_layout}: before staging"
        );

        let staged = layout::stage_beside(exe_dir, &profile, "fixture-cli").unwrap();
        assert_eq!(
            staged.is_some(),
            links,
            "{exe_layout}: staged only when needed"
        );
        let found = production_lookup(&exe, "fixture-cli").expect("reachable after staging");
        assert_eq!(
            std::fs::canonicalize(found).unwrap(),
            std::fs::canonicalize(&fixture).unwrap(),
            "{exe_layout}"
        );
        // Idempotent.
        assert_eq!(
            layout::stage_beside(exe_dir, &profile, "fixture-cli").unwrap(),
            staged
        );
    }
}

#[test]
fn staging_replaces_a_stale_entry_and_leaves_no_scratch_files() {
    let (_dir, profile, exe) = target("debug", BUILD_DIR);
    let exe_dir = exe.parent().unwrap();
    std::fs::write(profile.join("fixture-cli"), b"fixture").unwrap();
    let link = exe_dir.join("fixture-cli");
    std::fs::write(&link, b"stale regular file").unwrap();
    layout::stage_beside(exe_dir, &profile, "fixture-cli").unwrap();
    assert_eq!(std::fs::read(&link).unwrap(), b"fixture");
    let names: Vec<_> = std::fs::read_dir(exe_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(
        names,
        [OsString::from("fixture-cli")],
        "no scratch link left"
    );
}
