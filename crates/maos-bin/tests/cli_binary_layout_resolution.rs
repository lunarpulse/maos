#![cfg(feature = "network")]
#![forbid(unsafe_code)]

//! `resolve_cli_binary` is production code: it looks in the executable's own
//! directory, that directory's parent, then `$PATH` — nothing that depends on
//! how Cargo lays out a test build. An in-process test finds the prebuilt
//! `worker-cli-fixture` because the HARNESS links it beside the test executable
//! (`fixtures::profile_binary_beside_test`), whichever Cargo layout put the
//! executable there: `<profile>/deps/<test>-<hash>` (<= 1.99, the parent lookup
//! already reaches `<profile>`) or `<profile>/build/<pkg>/<hash>/out/<test>-<hash>`
//! (>= 1.100, four levels below it).

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "../../../tests/harness/fixtures.rs"]
mod fixtures;

const CHILD_EXPECTS: &str = "MAOS_CLI_LAYOUT_RELOCATED_EXPECTS";

#[test]
fn a_test_binary_resolves_the_fixture_the_harness_staged_beside_it() {
    let expected = fixtures::profile_binary_beside_test("worker-cli-fixture");
    let resolved = maos_bin::worker_spawn::resolve_cli_binary("worker-cli-fixture")
        .expect("resolve_cli_binary finds the fixture beside the test executable");
    assert_eq!(
        std::fs::canonicalize(&resolved).expect("resolved path exists"),
        std::fs::canonicalize(&expected).expect("fixture path exists"),
        "resolved {resolved}, expected {}",
        expected.display()
    );
    // Found by the production daemon-sibling lookup (the test executable's own
    // directory or its parent), not by luck of `$PATH`.
    let exe = std::env::current_exe().expect("test executable");
    let dir = exe.parent().expect("test executable directory");
    let found_in = Path::new(&resolved).parent().expect("resolved directory");
    assert!(
        found_in == dir || Some(found_in) == dir.parent(),
        "resolved {resolved} outside the executable's directory {} and its parent",
        dir.display()
    );
}

#[test]
fn an_unknown_command_names_what_was_checked() {
    let error = maos_bin::worker_spawn::resolve_cli_binary("no-such-worker-cli-layout-probe")
        .expect_err("an absent command is refused");
    assert!(
        error.contains("daemon-sibling") && error.contains("$PATH"),
        "{error}"
    );
}

/// A binary installed under a directory that merely happens to be named `out`
/// must not widen the lookup: the intended `$PATH` entry wins over a same-named
/// file four directories above the executable (e.g. an attacker-writable
/// `/tmp/worker-cli-fixture` for `/tmp/<owned>/v1/bin/out/maos`). The executable
/// is relocated into such a directory and re-run in a child.
#[test]
fn a_non_cargo_out_directory_does_not_widen_the_lookup() {
    if let Some(expected) = std::env::var_os(CHILD_EXPECTS) {
        let resolved = maos_bin::worker_spawn::resolve_cli_binary("worker-cli-fixture")
            .expect("the PATH entry resolves");
        assert_eq!(PathBuf::from(resolved), PathBuf::from(expected));
        return;
    }
    let root = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("scratch root");
    let out = root.path().join("owned/v1/bin/out");
    std::fs::create_dir_all(&out).expect("create the lookalike out directory");
    // Four directories above `out`, where the retired widening looked.
    let hijack = root.path().join("worker-cli-fixture");
    std::fs::write(&hijack, b"#!/bin/sh\nexit 99\n").expect("plant the hijack file");
    let path_dir = root.path().join("path-entry");
    std::fs::create_dir(&path_dir).expect("create the PATH entry");
    let intended = path_dir.join("worker-cli-fixture");
    std::fs::write(&intended, b"#!/bin/sh\nexit 0\n").expect("plant the intended binary");

    let exe = std::env::current_exe().expect("test executable");
    let relocated = out.join("relocated-test");
    std::fs::hard_link(&exe, &relocated)
        .or_else(|_| std::fs::copy(&exe, &relocated).map(drop))
        .expect("relocate the test executable");
    let output = Command::new(&relocated)
        .args([
            "--exact",
            "a_non_cargo_out_directory_does_not_widen_the_lookup",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_EXPECTS, &intended)
        .env("PATH", &path_dir)
        .output()
        .expect("run the relocated test executable");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed;"),
        "relocated child must resolve the PATH entry {}: {stdout}\n{}",
        intended.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}
