//! Story 15-3 T10 — the verb-table completeness control (AC4; rulings F6 and
//! F13; enrolled by name in `.github/workflows/discipline.yml`).
//!
//! What this test pins, and how it is allowed to fail:
//!
//! - **Equality in both directions (F6).** The dispatcher arms in
//!   `src/main.rs` are keyed on `verbs::VerbName` resolved from `verbs::VERBS`
//!   (lookup-then-dispatch), so table↔dispatcher divergence is unrepresentable
//!   rather than merely tested — and this file enforces what remains:
//!   - *a verb in a dispatcher but not the table* can only enter through a new
//!     `VerbName` variant, and [`expected_argv`]'s match has **no `_` arm** —
//!     this test target then fails to compile (proven red; transcript in the
//!     story file);
//!   - *a table row with no dispatch* makes `src/main.rs`'s exhaustive
//!     `main()` matches fail to compile, and because every spawn here uses
//!     `env!("CARGO_BIN_EXE_maos")`, cargo builds that bin for this test
//!     target — so the failure lands on this test too (proven red both ways).
//! - **Non-empty + exact count.** A vacuous green is the failure mode this
//!   story exists to close: the verb-row count and the one-shot-mode count are
//!   asserted exactly.
//! - **Both feature sets.** The table is feature-independent (per-row
//!   `builds`, F6), so the surface assertions hold under the default build AND
//!   under `--no-default-features`; the in-process dispatch assertions flip on
//!   [`verbs::CURRENT_BUILD`], covering each build's view.
//! - **No process-global environment.** Every spawned invocation
//!   (`--help`/`-h`/`help`/`--version`/unknown verb) exits inside
//!   `verbs::dispatch` before any MAOS_HOME / Transparency Log / daemon state
//!   is touched, so this test needs no env lock — the story explicitly forbids
//!   adding a fifth ad-hoc copy of the env scaffolding to `crates/maos-bin/tests/`.

#[path = "../../../tests/harness/doorless_home.rs"]
mod doorless_home;
#[path = "../src/verbs.rs"]
mod verbs;

use std::{
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

/// The expected argv token per [`verbs::VerbName`] — deliberately an
/// exhaustive match WITHOUT a `_` arm: a new variant (the shape "a verb in a
/// dispatcher but not the table" must take) fails this file's compilation.
fn expected_argv(name: verbs::VerbName) -> &'static str {
    match name {
        verbs::VerbName::Init => "init",
        verbs::VerbName::Run => "run",
        verbs::VerbName::Shell => "shell",
        verbs::VerbName::Audit => "audit",
        verbs::VerbName::Traceback => "traceback",
        verbs::VerbName::Backup => "backup",
        verbs::VerbName::Install => "install",
        verbs::VerbName::Purge => "purge",
    }
}

/// Every variant, spelled out — kept in sync with [`expected_argv`] (the
/// compiler enforces both against the enum).
const ALL_NAMES: [verbs::VerbName; 8] = [
    verbs::VerbName::Init,
    verbs::VerbName::Run,
    verbs::VerbName::Shell,
    verbs::VerbName::Audit,
    verbs::VerbName::Traceback,
    verbs::VerbName::Backup,
    verbs::VerbName::Install,
    verbs::VerbName::Purge,
];

fn maos() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_maos"));
    // Story 16-1 (D-16-1-Q) — a bare `maos` IS the shell root, and the shell
    // root is a door root: isolate it from the developer's home or a real
    // `control.json` turns this response-path probe into a live daemon boot
    // that binds the operator door. `HOME`, never `MAOS_HOME` — these
    // invocations exit inside `verbs::dispatch` and must touch no stores at
    // all; the empty home simply configures no door.
    cmd.env("HOME", doorless_home::doorless_home())
        .env("XDG_DATA_HOME", doorless_home::doorless_xdg_data_home());
    cmd
}

/// Run a CLI response-path assertion under the contract's sub-second bound.
/// A regression that boots the daemon is killed and fails this test rather
/// than hanging the CI worker.
fn output_with_deadline(command: &mut Command) -> Output {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn maos");
    let deadline = Instant::now() + Duration::from_millis(900);
    loop {
        if child.try_wait().expect("poll maos").is_some() {
            return child.wait_with_output().expect("capture maos output");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("maos response exceeded the 900 ms contract");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn rendered_help() -> Vec<u8> {
    let mut buf = Vec::new();
    verbs::print_help(&mut buf);
    buf
}

fn builds_tag(builds: &[verbs::Build]) -> String {
    format!(
        "[{}]",
        builds
            .iter()
            .map(|build| build.label())
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn verbs_of_build(build: verbs::Build) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = verbs::VERBS
        .iter()
        .filter(|verb| verb.builds.contains(&build))
        .map(|verb| verb.argv)
        .collect();
    names.sort_unstable();
    names
}

#[test]
fn table_is_non_empty_and_exactly_the_expected_surface() {
    assert!(
        !verbs::VERBS.is_empty(),
        "the verb table must not be empty — a vacuous green is the failure mode"
    );
    assert_eq!(verbs::VERBS.len(), 8, "exact expected verb count");
    for name in ALL_NAMES {
        let rows: Vec<&verbs::Verb> = verbs::VERBS
            .iter()
            .filter(|verb| verb.name == name)
            .collect();
        assert_eq!(rows.len(), 1, "{name:?} must have exactly one row");
        assert_eq!(rows[0].argv, expected_argv(name), "{name:?} argv mismatch");
        assert!(!rows[0].summary.is_empty(), "{name:?} must carry a summary");
        assert!(
            !rows[0].builds.is_empty(),
            "{name:?} must list at least one build"
        );
    }
    let mut tokens: Vec<&str> = verbs::VERBS.iter().map(|verb| verb.argv).collect();
    tokens.sort_unstable();
    tokens.dedup();
    assert_eq!(
        tokens.len(),
        verbs::VERBS.len(),
        "argv tokens must be unique"
    );
    // Both feature sets, from the per-row `builds` (F6) — identical under
    // either compilation of this test.
    assert_eq!(
        verbs_of_build(verbs::Build::AirGap),
        vec!["audit", "backup", "init", "install", "purge", "run"],
        "air-gap build surface"
    );
    assert_eq!(
        verbs_of_build(verbs::Build::Network),
        vec!["audit", "init", "purge", "run", "shell", "traceback"],
        "network build surface"
    );
}

#[test]
fn dispatch_resolves_this_builds_rows_and_rejects_the_others() {
    for verb in verbs::VERBS {
        let outcome = verbs::dispatch(&[verb.argv.to_string()]);
        if verb.builds.contains(&verbs::CURRENT_BUILD) {
            assert_eq!(
                outcome,
                verbs::Outcome::Verb(verb),
                "{verb:?} is a row of this build and must dispatch"
            );
        } else {
            assert_eq!(
                outcome,
                verbs::Outcome::Unknown(verb.argv.to_string()),
                "{verb:?} is not a row of this build and must resolve Unknown"
            );
        }
    }
    // Meta-flags resolve in BOTH builds (F5).
    for flag in ["--help", "-h", "help"] {
        assert_eq!(verbs::dispatch(&[flag.to_string()]), verbs::Outcome::Help);
    }
    for flag in ["--version", "-V"] {
        assert_eq!(
            verbs::dispatch(&[flag.to_string()]),
            verbs::Outcome::Version
        );
    }
    assert_eq!(verbs::dispatch(&[]), verbs::Outcome::NoArgs);
}

#[test]
fn one_shot_mode_table_is_complete_and_frozen() {
    assert!(!verbs::MAOS_ONE_SHOT_MODES.is_empty());
    assert_eq!(
        verbs::MAOS_ONE_SHOT_MODES.len(),
        32,
        "exact expected MAOS_ONE_SHOT mode count — Story 16-1 (D-16-1-A) removed the 16 door/read modes; re-measure main.rs's one-shot block before bumping"
    );
    let mut sorted = verbs::MAOS_ONE_SHOT_MODES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        verbs::MAOS_ONE_SHOT_MODES.len(),
        "one-shot modes must be unique"
    );
    assert_eq!(
        sorted,
        verbs::MAOS_ONE_SHOT_MODES.to_vec(),
        "one-shot modes must stay sorted (diff-friendly roster)"
    );
    // Epic 20's exit-block tokens — the contract's own example rows.
    assert!(verbs::MAOS_ONE_SHOT_MODES.contains(&"registry-server"));
    assert!(verbs::MAOS_ONE_SHOT_MODES.contains(&"uninstall"));
    for requested in verbs::MAOS_ONE_SHOT_MODES {
        assert_eq!(
            verbs::resolve_one_shot_mode(requested)
                .expect("every table row resolves")
                .as_str(),
            *requested
        );
    }
    assert!(
        verbs::resolve_one_shot_mode("not-a-real-mode").is_none(),
        "unchecked environment strings must never enter runtime dispatch"
    );
}

#[test]
fn help_output_is_exactly_the_frozen_table() {
    let expected = rendered_help();
    assert!(
        expected.starts_with(b"VERBS:\n"),
        "help must open with the VERBS: section header"
    );
    let text = String::from_utf8(expected.clone()).expect("help output is UTF-8");
    assert!(
        text.contains("\nMAOS_ONE_SHOT MODES:\n"),
        "help must carry the MAOS_ONE_SHOT MODES: section header"
    );
    // The contract's three example rows, byte-for-byte.
    assert!(text.contains("\n  init          [air-gap,network]  Initialise a MAOS home\n"));
    assert!(text.contains("\n  run           [air-gap,network]  Run a worker\n"));
    assert!(text.contains("\n  shell         [network]          Interactive shell\n"));
    // Every verb of BOTH builds is listed in BOTH builds' help, annotated.
    for verb in verbs::VERBS {
        let line = format!(
            "\n  {:<14}{:<17}  {}\n",
            verb.argv,
            builds_tag(verb.builds),
            verb.summary
        );
        assert!(
            text.contains(&line),
            "help is missing the row for {verb:?}: {line:?}"
        );
    }
    for flag in ["--help", "-h", "help"] {
        let out = output_with_deadline(maos().arg(flag));
        assert!(
            out.status.success(),
            "`maos {flag}` must exit 0, got {out:?}"
        );
        assert_eq!(
            out.stdout, expected,
            "`maos {flag}` stdout must be exactly the rendered table"
        );
        assert!(
            out.stderr.is_empty(),
            "`maos {flag}` must emit nothing on stderr: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn help_output_parses_like_the_gate_parses_it() {
    let out = output_with_deadline(maos().arg("--help"));
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("UTF-8");

    // The frozen parse rules: a line whose full trimmed text is exactly
    // `VERBS:` / `MAOS_ONE_SHOT MODES:` opens a section; inside a section,
    // every line beginning with exactly two spaces is an entry whose first
    // whitespace-delimited token is the name; a section ends at the first
    // blank line or first line beginning with a non-space character.
    #[derive(PartialEq, Clone, Copy, Debug)]
    enum Section {
        Header,
        Verbs,
        Modes,
    }
    let mut section = Section::Header;
    let mut parsed_verbs: Vec<&str> = Vec::new();
    let mut parsed_modes: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        match section {
            Section::Header => {
                if trimmed == "VERBS:" {
                    section = Section::Verbs;
                } else if trimmed == "MAOS_ONE_SHOT MODES:" {
                    section = Section::Modes;
                } else {
                    panic!("stdout must carry only the frozen sections; got {line:?}");
                }
            }
            Section::Verbs | Section::Modes => {
                if line.is_empty() || !line.starts_with(' ') {
                    assert_eq!(
                        section,
                        Section::Verbs,
                        "no content may follow the mode list"
                    );
                    assert_eq!(
                        trimmed, "MAOS_ONE_SHOT MODES:",
                        "only the mode header may follow"
                    );
                    section = Section::Modes;
                    continue;
                }
                assert!(
                    line.starts_with("  ") && !line.starts_with("   "),
                    "entries are indented by exactly two spaces: {line:?}"
                );
                let token = line.split_whitespace().next().expect("entry has a token");
                if section == Section::Verbs {
                    parsed_verbs.push(token);
                } else {
                    parsed_modes.push(token);
                }
            }
        }
    }
    let expected_verbs: Vec<&str> = verbs::VERBS.iter().map(|verb| verb.argv).collect();
    assert_eq!(
        parsed_verbs, expected_verbs,
        "parsed verbs must equal the table"
    );
    assert_eq!(
        parsed_modes,
        verbs::MAOS_ONE_SHOT_MODES.to_vec().as_slice(),
        "parsed modes must equal the mode table"
    );
}

#[test]
fn version_flag_prints_the_version_line() {
    let mut expected = Vec::new();
    verbs::print_version(&mut expected);
    assert_eq!(
        String::from_utf8(expected.clone()).unwrap(),
        format!("maos {}\n", env!("CARGO_PKG_VERSION"))
    );
    for flag in ["--version", "-V"] {
        let out = output_with_deadline(maos().arg(flag));
        assert!(
            out.status.success(),
            "`maos {flag}` must exit 0, got {out:?}"
        );
        assert_eq!(
            out.stdout, expected,
            "`maos {flag}` stdout must be the version line"
        );
        assert!(
            out.stderr.is_empty(),
            "`maos {flag}` must emit nothing on stderr: {:?}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn unknown_verb_exits_non_zero_with_the_table() {
    let expected = rendered_help();
    let out = output_with_deadline(maos().arg("eval"));
    assert!(
        !out.status.success(),
        "`maos eval` must exit non-zero instead of hanging, got {out:?}"
    );
    assert_eq!(out.status.code(), Some(1), "`maos eval` must exit 1");
    assert_eq!(
        out.stdout, expected,
        "an unknown verb must print the usage table on stdout"
    );
    assert!(
        !out.stderr.is_empty(),
        "an unknown verb must diagnose on stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}
