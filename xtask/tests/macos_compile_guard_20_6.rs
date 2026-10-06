//! Story 20-6 — the path-filtered macOS compile guard in `discipline.yml`.
//!
//! Two kinds of proof, both read from the shipped workflow rather than from a
//! copy of it:
//!
//! 1. STRUCTURE — `macos-check` runs on the same runner label as `release.yml`'s
//!    `aarch64-apple-darwin` leg, is gated on `macos-scope`'s output, checks the
//!    whole workspace for the darwin target, and is the ONLY macOS job (D-15-4-F's
//!    cost ruling stands everywhere else). `aggregate` depends on both jobs and
//!    says "NOT APPLICABLE" out loud when the guard is skipped (AC3).
//! 2. BEHAVIOUR — the `macos-scope` step's shell script is EXTRACTED from the
//!    YAML and EXECUTED in throwaway git repositories against planted diffs, so
//!    "a change touching none of the filtered paths does not start a macOS runner"
//!    (AC2) is observed, not asserted about a regex. So is the aggregate
//!    disposition step.
//!
//! What this file cannot prove — that the darwin `cargo check` itself reds on one
//! of the three historical breaks — needs the darwin target and a macOS C
//! toolchain; that falsifier is recorded in the story evidence
//! (`20-6-evidence/`) and re-proven by CI.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_yaml::{Mapping, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn load(name: &str) -> Value {
    let path = root().join(".github/workflows").join(name);
    serde_yaml::from_str(&fs::read_to_string(&path).expect("read workflow"))
        .expect("parse workflow")
}

fn get<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .as_mapping()
        .and_then(|m: &Mapping| m.get(Value::String(name.to_string())))
}

fn key<'a>(value: &'a Value, name: &str) -> &'a Value {
    get(value, name).unwrap_or_else(|| panic!("missing key {name}"))
}

fn job<'a>(workflow: &'a Value, name: &str) -> &'a Value {
    key(key(workflow, "jobs"), name)
}

fn steps(job: &Value) -> &[Value] {
    key(job, "steps").as_sequence().expect("steps")
}

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_sequence()
        .expect("sequence")
        .iter()
        .filter_map(Value::as_str)
        .collect()
}

fn step_by<'a>(job: &'a Value, field: &str, wanted: &str) -> &'a Value {
    steps(job)
        .iter()
        .find(|s| get(s, field).and_then(Value::as_str) == Some(wanted))
        .unwrap_or_else(|| panic!("no step with {field}={wanted}"))
}

fn script(job: &Value, field: &str, wanted: &str) -> String {
    key(step_by(job, field, wanted), "run")
        .as_str()
        .expect("run script")
        .to_string()
}

// ─── structure ───────────────────────────────────────────────────────────────

#[test]
fn macos_check_runs_on_release_yamls_darwin_runner_and_checks_the_whole_workspace() {
    let discipline = load("discipline.yml");
    let release = load("release.yml");
    let check = job(&discipline, "macos-check");

    // The runner label is DERIVED from release.yml's darwin matrix leg, so the
    // guard can never compile under a different image than the artifact it
    // protects: moving one without the other reds this test.
    let darwin_leg = key(
        key(key(job(&release, "build"), "strategy"), "matrix"),
        "include",
    )
    .as_sequence()
    .expect("release matrix include")
    .iter()
    .find(|leg| get(leg, "target").and_then(Value::as_str) == Some("aarch64-apple-darwin"))
    .expect("release.yml keeps its aarch64-apple-darwin leg");
    assert_eq!(
        key(check, "runs-on").as_str(),
        key(darwin_leg, "os").as_str(),
        "macos-check must use release.yml's darwin runner label"
    );
    assert!(
        get(check, "strategy").is_none(),
        "one macOS runner, never a matrix: D-15-4-F's cost ruling"
    );
    assert!(
        key(check, "timeout-minutes").as_u64().is_some(),
        "every job carries a ceiling (E16-A2)"
    );
    assert!(
        get(check, "continue-on-error").is_none(),
        "a guard that cannot fail is not a guard (rule 11)"
    );

    assert_eq!(key(check, "needs").as_str(), Some("macos-scope"));
    assert_eq!(
        key(check, "if").as_str(),
        Some("needs.macos-scope.outputs.applicable == 'true'"),
        "the macOS runner starts only when macos-scope says applicable"
    );

    let toolchain = steps(check)
        .iter()
        .find(|s| {
            get(s, "uses")
                .and_then(Value::as_str)
                .is_some_and(|u| u.starts_with("dtolnay/rust-toolchain"))
        })
        .expect("toolchain step");
    assert_eq!(
        key(key(toolchain, "with"), "targets").as_str(),
        Some("aarch64-apple-darwin")
    );

    let command = script(check, "name", "cargo check (aarch64-apple-darwin)");
    let tokens: Vec<&str> = command.split_whitespace().collect();
    assert_eq!(tokens[..2], ["cargo", "check"]);
    for required in [
        "--locked",
        "--workspace",
        "--all-targets",
        "--target",
        "aarch64-apple-darwin",
    ] {
        assert!(tokens.contains(&required), "missing {required}: {command}");
    }
    // The WHOLE workspace (operator ruling 2026-10-06): no `--exclude` and no
    // `-p` narrowing, in either spelling. A Linux-only crate is gated in its
    // own source (below), never dropped from the guard's command line.
    let narrowing: Vec<&str> = tokens
        .iter()
        .copied()
        .filter(|t| {
            ["--exclude", "-p", "--package"]
                .iter()
                .any(|flag| *t == *flag || t.starts_with(&format!("{flag}=")))
                || (t.starts_with("-p") && *t != "-p")
        })
        .collect();
    assert!(narrowing.is_empty(), "{narrowing:?} narrows: {command}");
}

#[test]
fn the_linux_only_exec_deps_crate_is_cfg_gated_in_source_not_excluded() {
    // `maos-exec-deps` (ELF parser, `libc::O_PATH`, procfs reopen) has no
    // macOS meaning. It compiles EMPTY off Linux, so the whole-workspace
    // darwin check above covers it; removing the gate is exactly the 4th
    // break the 20-6 preflight measured (E0425 `O_PATH`). Its integration test
    // is gated the same way, or `--all-targets` would fail to resolve it.
    for file in [
        "crates/maos-exec-deps/src/lib.rs",
        "crates/maos-exec-deps/tests/resolver_17_6.rs",
    ] {
        let source = fs::read_to_string(root().join(file)).expect(file);
        // Crate level = in the leading run of comments and inner attributes,
        // before the first item (a `#![cfg]` inside a `mod` gates only that).
        let header: Vec<&str> = source
            .lines()
            .map(str::trim)
            .take_while(|l| l.is_empty() || l.starts_with("//") || l.starts_with("#!["))
            .collect();
        assert!(
            header.contains(&"#![cfg(target_os = \"linux\")]"),
            "{file} must carry a crate-level #![cfg(target_os = \"linux\")]"
        );
    }
}

#[test]
fn macos_scope_is_a_cheap_linux_job_with_full_history() {
    let discipline = load("discipline.yml");
    let scope = job(&discipline, "macos-scope");
    assert_eq!(key(scope, "runs-on").as_str(), Some("ubuntu-latest"));
    assert!(key(scope, "timeout-minutes").as_u64().is_some());
    let checkout = steps(scope)
        .iter()
        .find(|s| get(s, "uses").and_then(Value::as_str) == Some("actions/checkout@v5"))
        .expect("checkout");
    assert_eq!(
        key(key(checkout, "with"), "fetch-depth").as_u64(),
        Some(0),
        "the diff base must be reachable"
    );
    let outputs = key(scope, "outputs");
    assert!(get(outputs, "applicable").is_some() && get(outputs, "reason").is_some());
}

/// Any string anywhere under `value` (scalar, sequence or mapping) mentions `needle`.
fn mentions(value: &Value, needle: &str) -> bool {
    match value {
        Value::String(s) => s.to_ascii_lowercase().contains(needle),
        Value::Sequence(items) => items.iter().any(|v| mentions(v, needle)),
        Value::Mapping(map) => map
            .iter()
            .any(|(k, v)| mentions(k, needle) || mentions(v, needle)),
        Value::Tagged(tagged) => mentions(&tagged.value, needle),
        _ => false,
    }
}

#[test]
fn macos_check_is_the_only_macos_job_and_the_release_dry_run_matrix_is_unchanged() {
    let discipline = load("discipline.yml");
    let jobs = key(&discipline, "jobs").as_mapping().expect("jobs");
    // A job runs on macOS if its `runs-on` names it directly, or is an
    // expression (`${{ matrix.os }}`) over a matrix that names it anywhere.
    let macos_jobs: Vec<&str> = jobs
        .iter()
        .filter(|(_, j)| {
            get(j, "runs-on").is_some_and(|r| {
                mentions(r, "macos")
                    || (mentions(r, "${{")
                        && get(j, "strategy")
                            .and_then(|s| get(s, "matrix"))
                            .is_some_and(|m| mentions(m, "macos")))
            })
        })
        .filter_map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(macos_jobs, ["macos-check"]);

    let matrix = key(
        key(job(&discipline, "release-dry-run"), "strategy"),
        "matrix",
    );
    assert_eq!(
        strings(key(matrix, "target")),
        ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"],
        "D-15-4-F stands for the release-dry-run matrix"
    );
}

#[test]
fn aggregate_depends_on_both_jobs_and_reports_not_applicable_loudly() {
    let discipline = load("discipline.yml");
    let aggregate = job(&discipline, "aggregate");
    let needs = strings(key(aggregate, "needs"));
    assert!(needs.contains(&"macos-scope") && needs.contains(&"macos-check"));
    assert_eq!(key(aggregate, "if").as_str(), Some("always()"));

    let disposition = script(
        aggregate,
        "name",
        "macOS guard disposition (Story 20-6 AC3)",
    );
    assert!(disposition.contains("NOT APPLICABLE (not a pass)"));
    assert!(disposition.contains("pre-tag rehearsal"));

    // The applicable-but-skipped failure is the LAST step, after the PR comment,
    // so the comment is refreshed instead of left stale by an early `exit 1`.
    let names: Vec<&str> = steps(aggregate)
        .iter()
        .filter_map(|s| get(s, "name").and_then(Value::as_str))
        .collect();
    let last = *names.last().expect("aggregate steps");
    assert_eq!(
        last,
        "Fail if the macOS guard was applicable but did not run"
    );
    let position = |name: &str| names.iter().position(|n| *n == name).expect(name);
    assert!(
        position("macOS guard disposition (Story 20-6 AC3)") < position("Post/update PR comment")
            && position("Post/update PR comment") < position(last)
    );
    let final_step = step_by(aggregate, "name", last);
    assert_eq!(
        key(final_step, "if").as_str(),
        Some("${{ steps.macos_guard.outputs.unran == 'true' }}")
    );
    assert_eq!(
        get(
            step_by(
                aggregate,
                "name",
                "macOS guard disposition (Story 20-6 AC3)"
            ),
            "id"
        )
        .and_then(Value::as_str),
        Some("macos_guard")
    );
}

fn pr_comment_script() -> String {
    let discipline = load("discipline.yml");
    key(
        key(
            step_by(
                job(&discipline, "aggregate"),
                "name",
                "Post/update PR comment",
            ),
            "with",
        ),
        "script",
    )
    .as_str()
    .expect("github-script body")
    .to_owned()
}

#[test]
fn the_pr_comment_never_calls_a_failed_filter_not_applicable() {
    let comment = pr_comment_script();
    // "not applicable" is rendered only behind scope=success AND applicable=false.
    assert!(comment.contains("mcsScope === 'success' && mcsApplicable === 'false'"));
    assert!(comment.contains("NOT APPLICABLE (not a pass)"));
    assert!(comment.contains("NOT RUN — macos-scope"));
    assert!(comment.contains("${mcsCell}"));
}

#[test]
fn the_pr_comment_script_reads_no_const_before_its_declaration() {
    // A `const` read before its declaration throws `ReferenceError` (temporal
    // dead zone), the github-script step fails, and `aggregate` reds on every
    // pull request where the guard RAN (frontier review F1: `mcsCell` called
    // `icon` above `const icon`). Same class as Story 8.16's undeclared `dt`.
    let script = pr_comment_script();
    let code: String = script
        .lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '$';
    let mut declared = 0;
    for (at, _) in code.match_indices("const ") {
        let rest = &code[at + 6..];
        // `const name =` or `const { key: name, other } =`.
        let names: Vec<String> = match rest.strip_prefix('{') {
            Some(pattern) => pattern[..pattern.find('}').expect("closed pattern")]
                .split(',')
                .map(|b| b.rsplit(':').next().unwrap_or(b).trim().to_owned())
                .collect(),
            None => vec![rest.chars().take_while(|c| is_ident(*c)).collect()],
        };
        let before = &code[..at];
        for name in names {
            assert!(
                !name.is_empty() && name.chars().all(is_ident),
                "unparsed const `{name}`"
            );
            declared += 1;
            // An identifier-bounded read, not a property (`x.name`) access.
            let early = before.match_indices(name.as_str()).find(|(i, _)| {
                let prev = before[..*i].chars().next_back();
                let next = before[i + name.len()..].chars().next();
                !prev.is_some_and(|c| is_ident(c) || c == '.') && !next.is_some_and(is_ident)
            });
            assert!(
                early.is_none(),
                "`{name}` is read on script line {} before `const {name}` (TDZ ReferenceError)",
                early.map_or(0, |(i, _)| before[..i].lines().count())
            );
        }
    }
    assert!(declared >= 4, "the script's consts were not found");
}

// ─── the disposition step, executed ──────────────────────────────────────────

/// (exit ok, combined output, `unran` output set)
fn run_disposition(
    scope_result: &str,
    applicable: &str,
    check_result: &str,
) -> (bool, String, bool) {
    let discipline = load("discipline.yml");
    let aggregate = job(&discipline, "aggregate");
    let body = script(
        aggregate,
        "name",
        "macOS guard disposition (Story 20-6 AC3)",
    );
    let dir = tempfile::tempdir().expect("tempdir");
    let summary = dir.path().join("summary");
    let outputs = dir.path().join("output");
    fs::write(&summary, "").unwrap();
    fs::write(&outputs, "").unwrap();
    let out = Command::new("bash")
        .args(["-c", &body])
        .env("SCOPE_RESULT", scope_result)
        .env("SCOPE_APPLICABLE", applicable)
        .env("SCOPE_REASON", "reason-under-test")
        .env("CHECK_RESULT", check_result)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .env("GITHUB_OUTPUT", &outputs)
        .output()
        .expect("run bash");
    let stdout =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    let unran = fs::read_to_string(&outputs).unwrap().contains("unran=true");
    (out.status.success(), stdout, unran)
}

#[test]
fn a_skipped_guard_is_not_applicable_never_a_silent_pass() {
    let (ok, text, unran) = run_disposition("success", "false", "skipped");
    assert!(ok && !unran);
    assert!(text.contains("NOT APPLICABLE (not a pass)"), "{text}");
    assert!(text.contains("reason-under-test"), "{text}");
    assert!(text.contains("stays mandatory"), "{text}");
}

#[test]
fn an_applicable_guard_that_did_not_run_is_flagged_for_the_failing_step() {
    let (ok, text, unran) = run_disposition("success", "true", "skipped");
    assert!(ok, "the step records; the LAST step fails: {text}");
    assert!(unran, "must set unran=true: {text}");
    assert!(text.contains("did not run"), "{text}");
}

#[test]
fn a_guard_that_ran_is_reported_as_ran_and_a_failure_is_not_softened() {
    let (ok, text, unran) = run_disposition("success", "true", "success");
    assert!(ok && !unran && text.contains("RAN and PASSED"), "{text}");
    let (ok, text, unran) = run_disposition("success", "true", "failure");
    assert!(
        ok && !unran,
        "the aggregate's own failure step owns the red: {text}"
    );
    assert!(text.contains("macos-check: failure"), "{text}");
    let (ok, text, unran) = run_disposition("failure", "", "skipped");
    assert!(
        ok && !unran && text.contains("macos-scope was failure"),
        "{text}"
    );
    let (_, text, unran) = run_disposition("cancelled", "", "skipped");
    assert!(
        !unran && text.contains("macos-scope was cancelled"),
        "{text}"
    );
}

// ─── the scope filter, executed against planted diffs ────────────────────────

const PORTABLE: &str = "pub fn add(a: u32, b: u32) -> u32 {\n    a + b\n}\n";

struct Scope {
    applicable: bool,
    reason: String,
    /// The raw `$GITHUB_OUTPUT` file the step wrote.
    outputs: String,
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn commit(dir: &Path, files: &[(&str, Option<&str>)], message: &str) -> String {
    for (path, content) in files {
        let full = dir.join(path);
        match content {
            Some(text) => {
                fs::create_dir_all(full.parent().unwrap()).unwrap();
                fs::write(&full, text).unwrap();
            }
            None => fs::remove_file(&full).unwrap(),
        }
    }
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", message]);
    git(dir, &["rev-parse", "HEAD"])
}

fn run_scope(dir: &Path, base: &str, head: &str) -> Scope {
    run_scope_event(dir, "push", base, head)
}

fn run_scope_event(dir: &Path, event: &str, base: &str, head: &str) -> Scope {
    let discipline = load("discipline.yml");
    let body = script(job(&discipline, "macos-scope"), "id", "scope");
    let outputs = dir.join("../github-output");
    let summary = dir.join("../github-summary");
    fs::write(&outputs, "").unwrap();
    fs::write(&summary, "").unwrap();
    let out = Command::new("bash")
        .current_dir(dir)
        .args(["-c", &body])
        .env("EVENT_NAME", event)
        .env("BASE_SHA", base)
        .env("HEAD_SHA", head)
        .env("GITHUB_OUTPUT", &outputs)
        .env("GITHUB_STEP_SUMMARY", &summary)
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "scope script failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = fs::read_to_string(&outputs).unwrap();
    let field = |name: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(&format!("{name}=")))
            .unwrap_or_else(|| panic!("no {name} output in {text:?}"))
            .to_string()
    };
    Scope {
        applicable: field("applicable") == "true",
        reason: field("reason"),
        outputs: text.clone(),
    }
}

/// Commit `base`, then `change`, and run the filter over exactly that diff.
fn scenario(base: &[(&str, &str)], change: &[(&str, Option<&str>)]) -> Scope {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let base_files: Vec<(&str, Option<&str>)> = base.iter().map(|(p, c)| (*p, Some(*c))).collect();
    let base_sha = commit(&dir, &base_files, "base");
    let head_sha = commit(&dir, change, "change");
    run_scope(&dir, &base_sha, &head_sha)
}

const MACOS_RS: &str = "crates/maos-kernel-core/src/security/sandbox/macos.rs";
const SANDBOX_MOD: &str = "crates/maos-kernel-core/src/security/sandbox/mod.rs";
const LIB: &str = "crates/foo/src/lib.rs";

#[test]
fn changes_touching_none_of_the_filtered_paths_start_no_macos_runner() {
    let base = [(LIB, PORTABLE), ("docs/a.md", "a\n"), ("README.md", "r\n")];
    for change in [
        vec![("docs/a.md", Some("b\n"))],
        vec![("README.md", Some("changed\n"))],
        vec![(
            LIB,
            Some("pub fn add(a: u32, b: u32) -> u32 {\n    a + b + 0\n}\n"),
        )],
        vec![("crates/foo/tests/t.rs", Some("#[cfg(test)]\nmod t {}\n"))],
        vec![(
            "crates/foo/src/other.rs",
            Some("// unix is only a word here, and so is windows\npub fn x() {}\n"),
        )],
        vec![(".github/workflows/ci-unrelated.yml", Some("name: x\n"))],
        // `std::os::raw` is portable — not an OS-typed module.
        vec![(
            "crates/foo/src/raw.rs",
            Some("pub type C = std::os::raw::c_int;\n"),
        )],
        // Frontier review F9: trees that are not workspace members (their own
        // fuzz/guest/fixture/spike workspaces) cannot change what
        // `cargo check --workspace` compiles, whatever they contain.
        vec![("spikes/story-x/bridge/Cargo.lock", Some("version = 4\n"))],
        vec![("crates/foo/fuzz/Cargo.toml", Some("[package]\n"))],
        vec![(
            "crates/foo/guests/g/src/lib.rs",
            Some("#[cfg(unix)]\nfn a() -> i32 { libc::O_PATH }\n"),
        )],
        vec![(
            "crates/foo/test-fixtures/probe/macos.rs",
            Some("fn a() {}\n"),
        )],
        vec![("templates/spirit-rust/Cargo.toml", Some("[package]\n"))],
        vec![(
            "xtask/tests/fixtures/p1/src/lib.rs",
            Some("use std::os::unix::fs::MetadataExt;\n"),
        )],
    ] {
        let scope = scenario(&base, &change);
        assert!(
            !scope.applicable,
            "{:?} must not start macOS: {}",
            change[0].0, scope.reason
        );
        assert!(scope.reason.contains("NOT a pass"), "{}", scope.reason);
    }
}

fn scope_regex(name: &str) -> String {
    let discipline = load("discipline.yml");
    let body = script(job(&discipline, "macos-scope"), "id", "scope");
    let prefix = format!("{name}='");
    body.lines()
        .find_map(|l| l.trim().strip_prefix(prefix.as_str()))
        .and_then(|rest| rest.strip_suffix('\''))
        .unwrap_or_else(|| panic!("{name}= not found in the macos-scope script"))
        .to_owned()
}

#[test]
fn no_workspace_member_is_treated_as_outside_the_workspace() {
    // The OUTSIDE_RE skip may only ever drop non-members: every member's
    // manifest (read from the root `Cargo.toml`) must still start the guard.
    let outside = regex::Regex::new(&scope_regex("OUTSIDE_RE")).expect("OUTSIDE_RE");
    let manifest: toml::Value = fs::read_to_string(root().join("Cargo.toml"))
        .expect("root Cargo.toml")
        .parse()
        .expect("toml");
    let members = manifest["workspace"]["members"]
        .as_array()
        .expect("workspace.members");
    assert!(members.len() > 10, "members not read");
    for member in members {
        let member = member.as_str().expect("member path");
        for file in ["Cargo.toml", "src/lib.rs", "src/main.rs", "build.rs"] {
            let path = format!("{member}/{file}");
            assert!(
                !outside.is_match(&path),
                "workspace member path {path} matches OUTSIDE_RE"
            );
        }
    }
    // And executed: a member manifest change starts the guard.
    let scope = scenario(
        &[("spirits/butler/Cargo.toml", "[package]\n")],
        &[(
            "spirits/butler/Cargo.toml",
            Some("[package]\nname = \"b\"\n"),
        )],
    );
    assert!(scope.applicable, "{}", scope.reason);
}

#[test]
fn every_linux_only_dependency_is_an_os_typed_api_for_the_filter() {
    // A crate a workspace manifest takes only under `cfg(target_os = "linux")`
    // does not exist (or, for `maos-exec-deps`, is empty) on macOS, so an
    // un-gated use of it anywhere is a darwin break the filter must see
    // (frontier review F3). Derived from the manifests, not a copied list.
    let api_re = scope_regex("API_RE");
    let mut linux_only = Vec::new();
    for entry in fs::read_dir(root().join("crates")).expect("crates/") {
        let manifest = entry.expect("entry").path().join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let parsed: toml::Value = text.parse().expect("member manifest");
        let Some(targets) = parsed.get("target").and_then(toml::Value::as_table) else {
            continue;
        };
        let deps_of = |table: &toml::Value| -> Vec<String> {
            table
                .get("dependencies")
                .and_then(toml::Value::as_table)
                .map(|d| d.keys().map(|k| k.replace('-', "_")).collect())
                .unwrap_or_default()
        };
        let mut on_linux = Vec::new();
        // Also present on macOS: plain deps, and target tables macOS satisfies.
        let mut on_macos = deps_of(&parsed);
        for (cfg, table) in targets {
            match cfg.replace(' ', "").as_str() {
                "cfg(target_os=\"linux\")" => on_linux.extend(deps_of(table)),
                "cfg(target_os=\"macos\")" | "cfg(unix)" => on_macos.extend(deps_of(table)),
                _ => {}
            }
        }
        linux_only.extend(on_linux.into_iter().filter(|d| !on_macos.contains(d)));
    }
    assert!(
        linux_only.iter().any(|d| d == "maos_exec_deps"),
        "manifests not read: {linux_only:?}"
    );
    for dep in linux_only {
        assert!(
            api_re.contains(&format!("{dep}::")),
            "API_RE misses the Linux-only dependency `{dep}::`: {api_re}"
        );
    }
}

#[test]
fn named_platform_sensitive_paths_start_the_guard() {
    let base = [(LIB, PORTABLE), (MACOS_RS, PORTABLE), ("Cargo.lock", "a\n")];
    for path in [
        MACOS_RS,
        "crates/anything/src/macos.rs",
        SANDBOX_MOD,
        "crates/maos-kernel-core/src/security/sandbox/t3/spawn.rs",
        "crates/maos-bin/src/purge.rs",
        "crates/maos-bin/src/operator_door.rs",
        "Cargo.lock",
        "Cargo.toml",
        "crates/foo/Cargo.toml",
        "crates/foo/build.rs",
        ".cargo/config.toml",
        "rust-toolchain.toml",
        ".github/workflows/release.yml",
    ] {
        let scope = scenario(&base, &[(path, Some("x = 1\n"))]);
        assert!(scope.applicable, "{path}: {}", scope.reason);
    }
}

#[test]
fn deleting_a_platform_file_still_counts() {
    let scope = scenario(
        &[(MACOS_RS, PORTABLE), (LIB, PORTABLE)],
        &[(MACOS_RS, None)],
    );
    assert!(scope.applicable, "{}", scope.reason);
}

#[test]
fn rust_source_with_platform_conditional_code_starts_the_guard() {
    let base = [(LIB, PORTABLE)];
    for (label, source) in [
        (
            "cfg(target_os",
            "#[cfg(target_os = \"linux\")]\nfn a() {}\n",
        ),
        ("cfg(unix)", "#[cfg(unix)]\nfn a() {}\n"),
        ("cfg(not(unix))", "#[cfg(not(unix))]\nfn a() {}\n"),
        ("cfg!(windows)", "fn a() -> bool { cfg!(windows) }\n"),
        (
            "cfg(any(...))",
            "#[cfg(any(target_os = \"macos\", target_os = \"linux\"))]\nfn a() {}\n",
        ),
        (
            "cfg_attr",
            "#[cfg_attr(target_os = \"linux\", derive(Debug))]\nstruct S;\n",
        ),
        // The purge.rs break: OS-typed API use with no cfg at the failing site.
        (
            "MetadataExt",
            "use std::os::unix::fs::MetadataExt;\nfn d(m: &std::fs::Metadata) -> u64 { m.dev() }\n",
        ),
        ("libc::", "fn a() -> i32 { libc::O_PATH }\n"),
        ("rustix::", "use rustix::fs::Dev;\n"),
        ("/proc/", "const P: &str = \"/proc/self/fd\";\n"),
        // Frontier review F4: `os::linux` nested in a grouped import, and the
        // macOS/Windows `std::os` modules.
        (
            "use std::{io, os::linux::…}",
            "use std::{io, os::linux::net::SocketAddrExt};\npub fn x() {}\n",
        ),
        (
            "use std::os::{fd::…, linux::…}",
            "use std::os::{\n    fd::AsRawFd,\n    linux::process::CommandExt,\n};\n",
        ),
        (
            "std::os::macos",
            "pub type T = std::os::macos::raw::stat;\n",
        ),
        // Frontier review F3: crates a manifest takes only under
        // `cfg(target_os = "linux")` (pinned against the manifests below).
        (
            "maos_exec_deps::",
            "pub fn x() { let _ = maos_exec_deps::resolve; }\n",
        ),
        ("landlock::", "use landlock::ABI;\n"),
        ("seccompiler::", "use seccompiler::SeccompAction;\n"),
    ] {
        let scope = scenario(&base, &[("crates/foo/src/new.rs", Some(source))]);
        assert!(scope.applicable, "{label}: {}", scope.reason);
        assert!(scope.reason.contains("new.rs"), "{label}: {}", scope.reason);
    }
}

#[test]
fn editing_a_file_that_already_contains_a_cfg_starts_the_guard() {
    // The epic's own wording: "any file containing `cfg(target_os` or
    // `cfg(unix)` changed in the diff" — the edited line itself is portable.
    let with_cfg = "#[cfg(unix)]\nfn a() {}\nfn b() -> u32 { 1 }\n";
    let edited = "#[cfg(unix)]\nfn a() {}\nfn b() -> u32 { 2 }\n";
    let scope = scenario(&[(LIB, with_cfg)], &[(LIB, Some(edited))]);
    assert!(scope.applicable, "{}", scope.reason);
}

#[test]
fn removing_the_last_cfg_from_a_file_starts_the_guard() {
    // After this edit the file contains no platform marker at all, so only the
    // removed line can tell the filter that a Linux-only item lost its gate.
    let before = "#[cfg(target_os = \"linux\")]\nfn a() {}\nfn b() {}\n";
    let after = "fn a() {}\nfn b() {}\n";
    let scope = scenario(&[(LIB, before)], &[(LIB, Some(after))]);
    assert!(scope.applicable, "{}", scope.reason);
    assert!(scope.reason.contains("adds or removes"), "{}", scope.reason);
}

#[test]
fn removing_a_wrapped_bare_unix_cfg_starts_the_guard() {
    // Frontier review F2: rustfmt leaves `unix` alone on its line; on the
    // removed-line pass that line carries a `-` prefix, which must be cut
    // before the anchored bare-predicate alternative can see it.
    let before = "#[cfg(all(\n    unix,\n    feature = \"some-quite-long-feature-name-here\"\n))]\nfn a() {}\nfn b() {}\n";
    let after = "fn a() {}\nfn b() {}\n";
    let scope = scenario(&[(LIB, before)], &[(LIB, Some(after))]);
    assert!(scope.applicable, "{}", scope.reason);
    assert!(scope.reason.contains("adds or removes"), "{}", scope.reason);
}

#[test]
fn discipline_yml_edits_start_the_guard_only_when_they_mention_macos() {
    let base = [(
        ".github/workflows/discipline.yml",
        "jobs:\n  a:\n    runs-on: ubuntu-latest\n",
    )];
    let unrelated = scenario(
        &base,
        &[(
            ".github/workflows/discipline.yml",
            Some("jobs:\n  a:\n    runs-on: ubuntu-latest\n  b:\n    runs-on: ubuntu-latest\n"),
        )],
    );
    assert!(!unrelated.applicable, "{}", unrelated.reason);
    let related = scenario(
        &base,
        &[(
            ".github/workflows/discipline.yml",
            Some("jobs:\n  a:\n    runs-on: ubuntu-latest\n  m:\n    runs-on: macos-latest\n"),
        )],
    );
    assert!(related.applicable, "{}", related.reason);
}

#[test]
fn an_unknown_diff_fails_open_and_runs_the_guard() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let head = commit(&dir, &[(LIB, Some(PORTABLE))], "only");
    for base in [
        "",
        "0000000000000000000000000000000000000000",
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    ] {
        let scope = run_scope(&dir, base, &head);
        assert!(scope.applicable, "base {base:?}: {}", scope.reason);
        assert!(scope.reason.contains("fail open"), "{}", scope.reason);
    }
}

#[test]
fn the_filter_reads_the_whole_pushed_range_not_the_last_commit() {
    // A push of two commits: the platform change is in the FIRST.
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let base = commit(&dir, &[(LIB, Some(PORTABLE))], "base");
    commit(&dir, &[(MACOS_RS, Some(PORTABLE))], "platform change");
    let head = commit(&dir, &[("docs/a.md", Some("a\n"))], "docs only");
    let scope = run_scope(&dir, &base, &head);
    assert!(scope.applicable, "{}", scope.reason);
    assert!(scope.reason.contains("macos.rs"), "{}", scope.reason);
}

#[test]
fn a_pull_request_is_diffed_against_its_merge_commits_first_parent_not_a_stale_base() {
    // `pull_request.base.sha` is the base tip when the PR was opened; `main`
    // has since moved (touching Cargo.lock). The PR itself only edits docs, so
    // charging it for main's commits would start a macOS runner it did not earn.
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("repo");
    fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    let opened_at = commit(
        &dir,
        &[(LIB, Some(PORTABLE)), ("docs/a.md", Some("a\n"))],
        "base",
    );
    git(&dir, &["checkout", "-q", "-b", "pr"]);
    commit(&dir, &[("docs/a.md", Some("pr edit\n"))], "docs only PR");
    git(&dir, &["checkout", "-q", "main"]);
    commit(&dir, &[("Cargo.lock", Some("moved on\n"))], "main moved");
    git(&dir, &["merge", "--no-ff", "-q", "pr", "-m", "merge pr"]);
    let merge = git(&dir, &["rev-parse", "HEAD"]);

    let stale = run_scope(&dir, &opened_at, &merge);
    assert!(
        stale.applicable,
        "sanity: the stale range DOES look platform-sensitive"
    );
    let scope = run_scope_event(&dir, "pull_request", &opened_at, &merge);
    assert!(!scope.applicable, "{}", scope.reason);

    // And a PR that really edits a platform file still starts the guard.
    git(&dir, &["checkout", "-q", "-b", "pr2"]);
    commit(&dir, &[(MACOS_RS, Some(PORTABLE))], "platform PR");
    git(&dir, &["checkout", "-q", "main"]);
    git(&dir, &["merge", "--no-ff", "-q", "pr2", "-m", "merge pr2"]);
    let merge2 = git(&dir, &["rev-parse", "HEAD"]);
    let scope = run_scope_event(&dir, "pull_request", &opened_at, &merge2);
    assert!(
        scope.applicable && scope.reason.contains("macos.rs"),
        "{}",
        scope.reason
    );
}

#[test]
fn non_ascii_paths_are_matched_not_c_quoted_past_the_filter() {
    let base = [(LIB, PORTABLE)];
    for path in ["crates/é/macos.rs", "crates/ü/Cargo.toml"] {
        let scope = scenario(&base, &[(path, Some("x = 1\n"))]);
        assert!(scope.applicable, "{path}: {}", scope.reason);
    }
    let scope = scenario(
        &base,
        &[(
            "crates/é/src/new.rs",
            Some("fn a() -> i32 { libc::O_PATH }\n"),
        )],
    );
    assert!(scope.applicable, "{}", scope.reason);
}

#[test]
fn a_path_with_a_newline_cannot_forge_the_step_outputs() {
    let scope = scenario(
        &[(LIB, PORTABLE)],
        &[(
            "crates/x/evil\napplicable=false.rs",
            Some("fn a() -> i32 { libc::O_PATH }\n"),
        )],
    );
    assert!(scope.applicable, "{}", scope.reason);
    let verdicts = scope
        .outputs
        .lines()
        .filter(|l| l.starts_with("applicable="))
        .count();
    assert_eq!(verdicts, 1, "exactly one verdict line: {:?}", scope.outputs);
}

#[test]
fn wrapped_cfg_attributes_and_other_target_predicates_start_the_guard() {
    let wrapped = "#[cfg(any(\n    target_os = \"linux\",\n    target_os = \"android\"\n))]\nfn a() {}\nfn b() -> u32 { 1 }\n";
    // A new file with a wrapped attribute…
    let scope = scenario(
        &[(LIB, PORTABLE)],
        &[("crates/foo/src/new.rs", Some(wrapped))],
    );
    assert!(scope.applicable, "wrapped, new file: {}", scope.reason);
    // …an edit to the body of a file whose ONLY marker is the wrapped attribute…
    let edited = wrapped.replace("{ 1 }", "{ 2 }");
    let scope = scenario(&[(LIB, wrapped)], &[(LIB, Some(edited.as_str()))]);
    assert!(scope.applicable, "wrapped, edited body: {}", scope.reason);
    // …un-gating it (every marker line removed)…
    let scope = scenario(
        &[(LIB, wrapped)],
        &[(LIB, Some("fn a() {}\nfn b() -> u32 { 1 }\n"))],
    );
    assert!(scope.applicable, "wrapped, ungated: {}", scope.reason);
    // …and the predicates that differ on darwin without being target_os.
    for source in [
        "#[cfg(target_env = \"gnu\")]\nfn a() {}\n",
        "#[cfg(target_arch = \"x86_64\")]\nfn a() {}\n",
        "#[cfg(not(\n    unix\n))]\nfn a() {}\n",
    ] {
        let scope = scenario(
            &[(LIB, PORTABLE)],
            &[("crates/foo/src/new.rs", Some(source))],
        );
        assert!(scope.applicable, "{source:?}: {}", scope.reason);
    }
}

#[test]
fn any_edit_to_the_macos_job_definitions_starts_the_guard_even_without_the_word_macos() {
    let workflow = |uses: &str, other: &str| {
        format!(
            "jobs:\n  a:\n    runs-on: ubuntu-latest\n    steps:\n      - run: {other}\n  macos-check:\n    runs-on: macos-latest\n    steps:\n      - uses: {uses}\n  z:\n    runs-on: ubuntu-latest\n"
        )
    };
    let path = ".github/workflows/discipline.yml";
    let base = workflow("dtolnay/rust-toolchain@v1", "one");
    // A change to an unrelated job, with the macOS job byte-identical: no runner.
    let unrelated = scenario(
        &[(path, &base)],
        &[(path, Some(&workflow("dtolnay/rust-toolchain@v1", "two")))],
    );
    assert!(!unrelated.applicable, "{}", unrelated.reason);
    // A change INSIDE the macOS job on a line that never says macos/darwin.
    let inside = scenario(
        &[(path, &base)],
        &[(path, Some(&workflow("dtolnay/rust-toolchain@v2", "one")))],
    );
    assert!(inside.applicable, "{}", inside.reason);
    assert!(
        inside.reason.contains("job definitions changed"),
        "{}",
        inside.reason
    );
}
