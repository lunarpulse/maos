//! Story 16-1 / D-16-1-Q — the local floor under the CI decoy.
//!
//! # Why this test exists
//!
//! Before Story 16-1, `$HOME/.maos` held nothing a concurrently running root
//! cared about, so a test could spawn `maos run` on the developer's real `HOME`
//! and nothing collided. After this story, `$HOME/.maos/control.json` names ONE
//! loopback endpoint, and a second root on the same endpoint fails fast with
//! `EndpointInUse` — by design, because the alternative is two daemons quietly
//! serving one operator surface.
//!
//! That turns every root-spawning test that inherits the developer's `HOME`
//! into a machine-dependent red: green in CI (where `$HOME/.maos` does not
//! exist) and red on any machine where the developer has ever run `maos init`.
//! `f4_pairing` and `two_host_delegation_2b` run two roots on the real `HOME`
//! **by design** — they isolate `MAOS_AUDIT_DB`, not `MAOS_HOME`, because
//! `MAOS_HOME` redirects the Transparency Log they are asserting on.
//!
//! # Why it is a FLOOR and not the control
//!
//! "Does this test isolate the home for the command that spawns a root" has no
//! textual boundary: builders are split across helpers, `MAOS_HOME` is
//! sometimes set late, some fixtures use `env_clear`, and a bare `maos` is a
//! shell root too. A lint that tried to answer it would be a lint that lies in
//! both directions.
//!
//! So the division of labour is:
//! - **The binding control** is the CI decoy step in
//!   `.github/workflows/discipline.yml`: it creates a scratch `HOME`, runs
//!   `maos init`, holds a listener on the endpoint `control.json` names, and
//!   runs the suites. A test that inherits that `HOME` reds there with
//!   `EndpointInUse`. That control cannot be satisfied by accident.
//! - **This test** is the floor: a file that spawns a `maos` root and contains
//!   NO isolation literal at all has certainly not thought about it. That is
//!   decidable from the text, so it is worth failing fast and locally on.
//!
//! A file already carrying `HOME`/`MAOS_HOME`/`env_clear` is out of scope here
//! — whether it isolates the RIGHT command is the decoy's question, not this
//! one.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

/// Files that spawn a `maos` root, set no home themselves, and are
/// deliberately exempt. Every entry states WHY, because an allowlist without
/// reasons becomes the place failures go to be forgotten.
const ALLOWED: &[(&str, &str)] = &[
    (
        "crates/maos-bin/tests/cert_rotation_trigger_14_2a.rs",
        "Drives the `cohort-a2a-daemon` root through an explicit \
         MAOS_OPERATOR_HTTP_BIND + MAOS_OPERATOR_BEARER_TOKEN pair, so its \
         endpoint never comes from control.json and cannot collide with the \
         developer's home. It also scrapes the listening marker, which is what \
         proves the third door root still binds.",
    ),
    (
        "crates/maos-journey-test/tests/journey_butler.rs",
        "Spawns only through `Pty::spawn(&cmd, &world)`, whose child \
         environment IS `JourneyWorldBuilder::build`'s env map — and that map \
         inserts the scratch HOME for every journey consumer at once \
         (crates/maos-journey-test/src/lib.rs). There is no `Command` object \
         in this file to set an env var on, so the isolation genuinely lives \
         one level up rather than being absent.",
    ),
    (
        "crates/maos-journey-test/tests/journey_j3.rs",
        "Same shape as journey_butler.rs: `maos_bin()` yields a path string \
         and every spawn goes through `Pty::spawn`, so the home comes from \
         `JourneyWorldBuilder::build`'s env map.",
    ),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/<crate> has a workspace root")
        .to_path_buf()
}

/// Every `tests/*.rs` under `crates/`, sorted, read as text.
fn test_files(root: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let crates = std::fs::read_dir(root.join("crates")).expect("crates/ is readable");
    for crate_dir in crates {
        let crate_dir = crate_dir.expect("readable crate entry").path();
        let tests = crate_dir.join("tests");
        if !tests.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&tests).expect("tests/ is readable");
        for entry in entries {
            let path = entry.expect("readable test entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .expect("under the workspace root")
                .to_string_lossy()
                .replace('\\', "/");
            let body = std::fs::read_to_string(&path).expect("test file is UTF-8");
            files.push((relative, body));
        }
    }
    files.sort();
    files
}

/// Does this file start the `maos` binary at all? `maosctl` is not a root and
/// never binds the door.
fn spawns_a_root(body: &str) -> bool {
    body.contains("CARGO_BIN_EXE_maos\"")
        || body.contains("MAOS_BIN_PATH")
        || body.contains("maos_binary(")
        || body.contains("maos_bin(")
}

/// Does the file actually SET a home for its children?
///
/// ⚠ A substring match on `"HOME"` was the first shape of this predicate, and
/// it was wrong in the satisfying direction — the failure Epic-15 action A6
/// names. Measured: `f4_pairing.rs` and `verb_table_15_3.rs` cleared it on a
/// COMMENT mentioning `MAOS_HOME` while setting nothing, and two more files
/// could have been made to pass by adding a comment. A floor that a comment
/// can satisfy is not a floor.
///
/// So the predicate requires an ASSIGNMENT, in any of the four shapes this
/// tree actually uses: `.env("HOME", ..)` on a command, `insert("HOME", ..)`
/// into a child env map, `env::set_var("MAOS_HOME", ..)` for a fixture that
/// drives the surface in-process, or `env_clear()` to drop the inherited one.
///
/// It also follows one level of `#[path = "..."]` module include, because
/// that is how this tree shares a spawn harness: eight `maos-cli` contract
/// tests isolate through `tests/support/fixture_door.rs`, and allowlisting
/// all eight would turn a real control into eight exemptions. A file whose
/// isolation lives behind a CRATE dependency rather than a textual include
/// still belongs in [`ALLOWED`] with its reason written down.
fn isolates_a_home(body: &str) -> bool {
    const SETTERS: [&str; 7] = [
        ".env(\"HOME\"",
        ".env(\"MAOS_HOME\"",
        "insert(\"HOME\"",
        "insert(\"MAOS_HOME\"",
        "set_var(\"HOME\"",
        "set_var(\"MAOS_HOME\"",
        "env_clear",
    ];
    // Tolerate `rustfmt`'s line breaks inside the call: `.env(\n  "HOME",`.
    let squeezed: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    SETTERS.iter().any(|setter| squeezed.contains(setter))
}

/// `isolates_a_home`, extended through the file's `#[path = "..."]` includes.
fn isolates_a_home_directly_or_via_include(root: &Path, rel: &str, body: &str) -> bool {
    if isolates_a_home(body) {
        return true;
    }
    let Some(dir) = Path::new(rel).parent() else {
        return false;
    };
    body.split("#[path = \"")
        .skip(1)
        .filter_map(|tail| tail.split('"').next())
        .any(|include| {
            std::fs::read_to_string(root.join(dir).join(include))
                .map(|included| isolates_a_home(&included))
                .unwrap_or(false)
        })
}

#[test]
fn every_root_spawning_test_names_a_home() {
    let root = workspace_root();
    let files = test_files(&root);
    assert!(
        files.len() > 100,
        "the file walk collapsed to {} files — a floor that inspects nothing \
         passes vacuously, which is the exact failure Epic-15 action A6 names",
        files.len()
    );

    let root_spawners: Vec<&(String, String)> = files
        .iter()
        .filter(|(_, body)| spawns_a_root(body))
        .collect();
    assert!(
        root_spawners.len() >= 20,
        "only {} files were detected as spawning a `maos` root; the detection \
         predicate has rotted and this test is no longer a floor",
        root_spawners.len()
    );

    let offenders: Vec<&str> = root_spawners
        .iter()
        .filter(|(path, body)| {
            !isolates_a_home_directly_or_via_include(&root, path, body)
                && !ALLOWED.iter().any(|(allowed, _)| allowed == path)
        })
        .map(|(path, _)| path.as_str())
        .collect();

    assert!(
        offenders.is_empty(),
        "these files spawn a `maos` root and name no home at all, so on any \
         machine where `maos init` has ever run they inherit \
         `$HOME/.maos/control.json` and a second root on that endpoint fails \
         `EndpointInUse` — green in CI, red on a developer's machine. Give \
         each a per-test `HOME` (and a per-test `XDG_DATA_HOME`, or remove \
         it), or add it to ALLOWED with a reason:\n  {}",
        offenders.join("\n  ")
    );
}

/// An allowlist entry for a file that has since been fixed, deleted or renamed
/// is an exemption nobody is checking. It must red.
#[test]
fn the_allowlist_carries_no_stale_entry() {
    let root = workspace_root();
    for (path, reason) in ALLOWED {
        let full = root.join(path);
        assert!(
            full.is_file(),
            "allowlisted file {path} does not exist — drop the entry"
        );
        let body = std::fs::read_to_string(&full).expect("allowlisted file is UTF-8");
        assert!(
            spawns_a_root(&body),
            "allowlisted file {path} no longer spawns a `maos` root — drop the entry"
        );
        assert!(
            !isolates_a_home_directly_or_via_include(&root, path, &body),
            "allowlisted file {path} now names a home (directly or through a \
             `#[path]` include) — drop the exemption rather than keeping a \
             reason that no longer applies"
        );
        assert!(
            reason.len() > 40,
            "allowlist entry {path} has no real reason"
        );
    }
}
