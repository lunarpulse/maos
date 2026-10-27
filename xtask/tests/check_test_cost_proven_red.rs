//! Story `20-12` AC2 — proven-red vectors for `check-test-cost`.
//!
//! Every vector plants into a tempdir tree and reads it back through the SAME
//! `audit_with()` / `audit_jobs()` the CI job runs (the 15-1 / 17-5 proven-red
//! shape): the control is GREEN, each plant is one change asserted to red naming
//! its `file:line`, and the real tree at HEAD is GREEN. The tracked tree is never
//! mutated. Planted spawn calls are assembled at runtime so this file is not
//! itself a cargo-spawning test.
//!
//! Review 2026-10-23 vectors: `env!`/`var`/`var_os("CARGO")`, `rustc` and
//! `trybuild` triggers; a marked call directly inside a non-ignored test; the
//! `// cargo-query:` marker (green for `tree`, red for a compiling subcommand,
//! a non-cargo spawn or an empty reason); an empty `#[ignore]` reason; members
//! outside `crates/`; a symlink loop; an `[[expensive]]` job that never runs
//! ignored tests.

use std::fs;
use std::path::{Path, PathBuf};
use xtask::check_test_budget::{load_budgets, parse_budgets};
use xtask::check_test_cost::{audit_jobs, audit_with, expensive_filter, Report};

fn real_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under the workspace root")
        .to_path_buf()
}

/// `<head>("<arg>")`, spelled so this source does not contain the call.
fn call(head: &str, arg: &str) -> String {
    format!("{head}({q}{arg}{q})", q = '"')
}

/// `Command::new("cargo")`.
fn cargo_call() -> String {
    call("Command::new", "cargo")
}

const REGISTERED: &str = "\
default_seconds = 30
total_seconds = 300

[[expensive]]
binary = \"foo::bar\"
job = \"expensive-tests\"
reason = \"spawns cargo\"
owner = \"20-12\"
";

const EMPTY: &str = "default_seconds = 30\ntotal_seconds = 300\n";

/// A workspace whose root `Cargo.toml` lists `members`, with each
/// `(path, contents)` written under it.
fn workspace(members: &[&str], files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let list: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
    fs::write(
        dir.path().join("Cargo.toml"),
        format!("[workspace]\nmembers = [{}]\n", list.join(", ")),
    )
    .expect("workspace manifest");
    for (path, contents) in files {
        let path = dir.path().join(path);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, contents).expect("file");
    }
    dir
}

/// A one-crate tree: package `foo`, integration test `crates/foo/tests/bar.rs`.
fn tree(test_source: &str) -> tempfile::TempDir {
    workspace(
        &["crates/foo"],
        &[
            ("crates/foo/Cargo.toml", "[package]\nname = \"foo\"\n"),
            ("crates/foo/tests/bar.rs", test_source),
        ],
    )
}

fn audit(test_source: &str, register: &str) -> Report {
    let dir = tree(test_source);
    audit_with(
        dir.path(),
        &parse_budgets(register).expect("register parses"),
    )
}

fn assert_green(report: &Report) {
    assert!(
        report.findings.is_empty(),
        "expected GREEN, got {:#?}",
        report.findings
    );
}

fn assert_red_at(report: &Report, line: usize, needle: &str) {
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.file == "crates/foo/tests/bar.rs"
                && f.line == line
                && f.detail.contains(needle)),
        "expected a red at crates/foo/tests/bar.rs:{line} containing {needle:?}, got {:#?}",
        report.findings
    );
}

fn helper_test(marker: &str, ignore: &str) -> String {
    format!(
        "use std::process::Command;\n\
         \n\
         fn build() {{\n\
         {marker}    let _ = {call}.arg(\"build\").status();\n\
         }}\n\
         \n\
         #[test]\n\
         {ignore}fn builds() {{\n\
         \x20   build();\n\
         }}\n",
        call = cargo_call()
    )
}

/// A test whose body is `marker` then `spawn` then `next`, both lines indented.
/// The spawn is on line `3 + marker lines`.
fn inline_test(marker: &str, spawn: &str, next: &str) -> String {
    format!("#[test]\nfn spawns() {{\n{marker}    let _ = {spawn}\n        {next};\n}}\n")
}

#[test]
fn control_without_cargo_is_green() {
    let report = audit("#[test]\nfn plain() {\n    assert_eq!(1, 1);\n}\n", EMPTY);
    assert_green(&report);
    assert_eq!(report.cargo_call_sites, 0);
}

#[test]
fn planting_an_unmarked_cargo_call_reds_naming_its_line() {
    let report = audit(&helper_test("", ""), EMPTY);
    assert_eq!(report.cargo_call_sites, 1);
    assert_red_at(&report, 4, "without `// expensive: <reason>`");
}

#[test]
fn an_empty_marker_reason_reds() {
    let report = audit(&helper_test("    // expensive:\n", ""), EMPTY);
    assert_red_at(&report, 5, "`// expensive:` with an empty reason");
}

#[test]
fn every_compiler_spawn_trigger_reds_unmarked() {
    let triggers = [
        (
            format!("Command::new({})", call("env!", "CARGO")),
            "spawns cargo",
        ),
        (call("std::env::var", "CARGO"), "spawns cargo"),
        (call("std::env::var_os", "CARGO"), "spawns cargo"),
        (call("Command::new", "rustc"), "spawns rustc"),
        (
            format!("trybuild{sep}TestCases{sep}new()", sep = "::"),
            "spawns trybuild",
        ),
    ];
    for (spawn, needle) in triggers {
        let report = audit(&inline_test("", &spawn, ".len()"), EMPTY);
        assert_eq!(report.cargo_call_sites, 1, "{spawn}");
        assert_red_at(&report, 3, needle);
        assert_red_at(&report, 3, "without `// expensive: <reason>`");
    }
}

#[test]
fn a_use_of_trybuild_is_not_a_spawn() {
    let source = format!(
        "use trybuild{sep}TestCases;\n#[test]\nfn plain() {{}}\n",
        sep = "::"
    );
    let report = audit(&source, EMPTY);
    assert_green(&report);
    assert_eq!(report.cargo_call_sites, 0);
}

#[test]
fn a_marked_call_reached_by_a_non_ignored_test_reds_at_that_test() {
    let report = audit(
        &helper_test("    // expensive: builds the tree\n", ""),
        REGISTERED,
    );
    assert_red_at(
        &report,
        9,
        "test `builds` reaches the marked cargo call via `build`",
    );
}

#[test]
fn a_marked_call_directly_inside_a_non_ignored_test_reds() {
    let report = audit(
        &inline_test(
            "    // expensive: builds the tree\n",
            &cargo_call(),
            ".arg(\"build\").status()",
        ),
        EMPTY,
    );
    assert_red_at(
        &report,
        4,
        "test `spawns` spawns cargo but is not `#[ignore = \"expensive: …\"]`",
    );
}

#[test]
fn an_empty_ignore_reason_reds() {
    let source = helper_test(
        "    // expensive: builds the tree\n",
        "#[ignore = \"expensive:  \"]\n",
    );
    let report = audit(&source, REGISTERED);
    assert_red_at(
        &report,
        10,
        "test `builds` is `#[ignore = \"expensive:\"]` with an empty reason",
    );
}

#[test]
fn marked_ignored_and_registered_is_green_and_feeds_the_ci_filter() {
    let source = helper_test(
        "    // expensive: builds the tree\n",
        "#[ignore = \"expensive: builds the tree\"]\n",
    );
    let register = parse_budgets(REGISTERED).expect("register parses");
    let dir = tree(&source);
    let report = audit_with(dir.path(), &register);
    assert_green(&report);
    assert_eq!(
        expensive_filter(&register, &report),
        "(binary_id(foo::bar) & test(/(^|::)(builds)$/))"
    );
}

fn static_helper_test(ignore: &str) -> String {
    format!(
        "fn build() {{\n\
         \x20   // expensive: builds the tree\n\
         \x20   let _ = {call}.arg(\"build\").status();\n\
         }}\n\
         static FIXTURE: LazyLock<()> = LazyLock::new(build);\n\
         \n\
         #[test]\n\
         {ignore}fn uses_fixture() {{\n\
         \x20   let _ = &*FIXTURE;\n\
         }}\n",
        call = cargo_call()
    )
}

#[test]
fn a_single_line_static_connects_its_helper_to_an_ignored_test() {
    let source = format!(
        "static UNRELATED: LazyLock<()> = LazyLock::new(|| ());\n{}\n\
         #[test]\nfn unrelated() {{\n    let _ = &*UNRELATED;\n}}\n",
        static_helper_test("#[ignore = \"expensive: builds the tree\"]\n")
    );
    let report = audit(&source, REGISTERED);
    assert_green(&report);
    assert_eq!(report.cargo_call_sites, 1);
    assert_eq!(report.expensive_tests["foo::bar"], ["uses_fixture"]);
    // An unused static must not inherit references from the following items.
    let unused = source.replace("let _ = &*FIXTURE;", "assert!(true);");
    assert_red_at(&audit(&unused, REGISTERED), 4, "no test reaches `build`");
}

#[test]
fn an_ordinary_test_reaching_a_single_line_static_reds_at_that_test() {
    let report = audit(&static_helper_test(""), EMPTY);
    assert_red_at(
        &report,
        8,
        "test `uses_fixture` reaches the marked cargo call via `build`",
    );
}

#[test]
fn a_cargo_query_for_tree_stays_in_the_default_run() {
    let report = audit(
        &inline_test(
            "    // cargo-query: resolves the graph, compiles nothing\n",
            &cargo_call(),
            ".args([\"tree\", \"-p\", \"foo\"]).output()",
        ),
        EMPTY,
    );
    assert_green(&report);
    assert_eq!(report.cargo_query_sites, 1);
    assert!(report.expensive_tests.is_empty());
    // A leading cargo flag is skipped; `metadata` is a query too.
    let report = audit(
        &inline_test(
            "    // cargo-query: reads the workspace metadata\n",
            &format!("Command::new({})", call("env!", "CARGO")),
            ".args([\"--locked\", \"metadata\"]).output()",
        ),
        EMPTY,
    );
    assert_green(&report);
}

#[test]
fn a_cargo_query_that_compiles_reds() {
    let report = audit(
        &inline_test(
            "    // cargo-query: not really\n",
            &cargo_call(),
            ".args([\"build\"]).status()",
        ),
        EMPTY,
    );
    assert_red_at(&report, 4, "runs cargo subcommand `build`");
    let report = audit(
        &inline_test(
            "    // cargo-query: not really\n",
            &cargo_call(),
            ".args([\"+nightly\", \"public-api\"]).status()",
        ),
        EMPTY,
    );
    assert_red_at(&report, 4, "runs cargo subcommand `+nightly`");
}

#[test]
fn a_cargo_query_on_a_compiler_or_without_reason_reds() {
    let report = audit(
        &inline_test(
            "    // cargo-query: version probe\n",
            &call("Command::new", "rustc"),
            ".arg(\"tree\").status()",
        ),
        EMPTY,
    );
    assert_red_at(&report, 4, "on a `rustc` spawn");
    let report = audit(
        &inline_test(
            "    // cargo-query:\n",
            &cargo_call(),
            ".arg(\"tree\").output()",
        ),
        EMPTY,
    );
    assert_red_at(&report, 4, "`// cargo-query:` with an empty reason");
}

#[test]
fn an_unregistered_expensive_binary_reds() {
    let source = helper_test(
        "    // expensive: builds the tree\n",
        "#[ignore = \"expensive: builds the tree\"]\n",
    );
    let report = audit(&source, EMPTY);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.detail.contains("foo::bar has expensive-ignored tests")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn a_stale_register_row_reds() {
    let report = audit("#[test]\nfn plain() {}\n", REGISTERED);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.detail.contains("[[expensive]] foo::bar names no test")),
        "{:#?}",
        report.findings
    );
}

#[test]
fn a_comment_that_mentions_cargo_is_not_a_call() {
    let source = format!("// {}\n#[test]\nfn plain() {{}}\n", cargo_call());
    assert_green(&audit(&source, EMPTY));
}

#[test]
fn every_workspace_member_tests_dir_is_scanned() {
    let source = helper_test("", "");
    let dir = workspace(
        &["crates/foo", "spirits/*"],
        &[
            ("crates/foo/Cargo.toml", "[package]\nname = \"foo\"\n"),
            ("spirits/bar/Cargo.toml", "[package]\nname = \"bar\"\n"),
            ("spirits/bar/tests/baz.rs", source.as_str()),
        ],
    );
    let report = audit_with(dir.path(), &parse_budgets(EMPTY).unwrap());
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.file == "spirits/bar/tests/baz.rs" && f.line == 4),
        "{:#?}",
        report.findings
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_loop_is_skipped() {
    let dir = tree("#[test]\nfn plain() {}\n");
    std::os::unix::fs::symlink(
        dir.path().join("crates/foo"),
        dir.path().join("crates/foo/tests/loop"),
    )
    .expect("symlink");
    let report = audit_with(dir.path(), &parse_budgets(EMPTY).unwrap());
    assert_green(&report);
    assert_eq!(report.files_scanned, 1);
}

const WORKFLOW: &str = "\
jobs:
  expensive-tests:
    runs-on: ubuntu-24.04
    steps:
      - name: Run the registered expensive tests
        run: cargo nextest run --workspace --run-ignored ignored-only -E \"$filter\"
  lazy-job:
    runs-on: ubuntu-24.04
    steps:
      # --ignored appears only in this comment
      - run: cargo nextest run --workspace
  gated-job:
    runs-on: ubuntu-24.04
    steps:
      - run: cargo run -p xtask -- check-foo --json
";

/// `REGISTERED` with its row's `job` replaced, audited against [`WORKFLOW`]
/// (and an `xtask/src/check_foo.rs` gate that passes `--ignored`).
fn audit_job(job: &str) -> Vec<xtask::check_test_cost::Finding> {
    let register = REGISTERED.replace("expensive-tests", job);
    let dir = workspace(
        &[],
        &[
            (".github/workflows/discipline.yml", WORKFLOW),
            (
                "xtask/src/check_foo.rs",
                "fn legs() { cmd.args([\"--\", \"filter\", \"--ignored\"]); }\n",
            ),
        ],
    );
    audit_jobs(dir.path(), &parse_budgets(&register).unwrap())
}

#[test]
fn an_expensive_job_must_run_ignored_tests() {
    assert!(audit_job("expensive-tests").is_empty());
    assert!(audit_job("gated-job").is_empty());
    let lazy = audit_job("lazy-job");
    assert!(
        lazy.iter().any(|f| f
            .detail
            .contains("names job `lazy-job`, whose steps never run ignored tests")),
        "{lazy:#?}"
    );
    let ghost = audit_job("ghost-job");
    assert!(
        ghost
            .iter()
            .any(|f| f.detail.contains("names job `ghost-job`, absent from")),
        "{ghost:#?}"
    );
}

#[test]
fn the_real_tree_is_green() {
    let root = real_root();
    let register = load_budgets(&root).expect("xtask/test-budgets.toml parses");
    let report = audit_with(&root, &register);
    assert_green(&report);
    assert!(
        report.files_scanned > 100,
        "scanned only {} files",
        report.files_scanned
    );
    let jobs = audit_jobs(&root, &register);
    assert!(jobs.is_empty(), "{jobs:#?}");
}
