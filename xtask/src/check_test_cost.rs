#![forbid(unsafe_code)]

//! Story 20-12 AC2 — `check-test-cost`: no test compiles the tree.
//!
//! Every test source under a workspace member's `tests/` dir (members read from
//! the root `Cargo.toml`, so `crates/*`, `spirits/*`, `examples/*` and `xtask`
//! alike) and the root `tests/` is scanned; `fixtures/` data trees, symlinks and
//! comment lines are skipped. A line that spawns a compiler — cargo
//! (`Command::new("cargo")`, `env!("CARGO")`, `var("CARGO")`, `var_os("CARGO")`),
//! `Command::new("rustc")`, or `trybuild` — must carry one of two markers on the
//! line directly above it:
//!
//! - `// expensive: <reason>` — and the tests that reach it must be
//!   `#[ignore = "expensive: <reason>"]`: the call's enclosing item is either such
//!   a test itself, or a helper/static that only such tests name. Every file
//!   holding an `expensive:` ignore must be registered in `xtask/test-budgets.toml`
//!   `[[expensive]]` (and every row must still name one), and the row's `job`
//!   must exist in `discipline.yml` and run ignored tests (`--run-ignored`,
//!   `--ignored` or `--include-ignored`, directly or through the `xtask` gate it
//!   invokes). The rows naming `expensive-tests` are exactly what that job runs.
//! - `// cargo-query: <reason>` (review 2026-10-23) — a cargo subcommand that
//!   compiles nothing (`tree`, `metadata`, `locate-project`, `pkgid`): the first
//!   non-flag string literal after the call, on its line or the next, must be one
//!   of those. Such tests stay in the default run (no `#[ignore]`). `rustc`,
//!   `trybuild` and every other cargo subcommand (`public-api` included) are
//!   expensive.
//!
//! An empty reason is a finding in both markers and in the `#[ignore]`.
//!
//! The scan is line-based over rustfmt-formatted sources (`check-fmt` enforces
//! the shape): an item ends at its own indentation with `}` (functions) or `;`
//! (static initializers), including items completed on their declaration line.

use crate::check_test_budget::{load_budgets, Budgets, BUDGETS_PATH};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const GATE: &str = "check-test-cost";
const MARKER: &str = "// expensive:";
const QUERY_MARKER: &str = "// cargo-query:";
const IGNORE: &str = "#[ignore = \"expensive:";
/// The CI job that runs every `[[expensive]]` row without a dedicated job.
pub const EXPENSIVE_JOB: &str = "expensive-tests";
/// The workflow every `[[expensive]]` job must live in.
pub const WORKFLOW: &str = ".github/workflows/discipline.yml";

/// A spawn trigger and what it runs (`cargo` triggers may be a cargo-query).
const TRIGGERS: [(&str, &str); 7] = [
    ("Command::new(\"cargo\")", "cargo"),
    ("env!(\"CARGO\")", "cargo"),
    ("var(\"CARGO\")", "cargo"),
    ("var_os(\"CARGO\")", "cargo"),
    ("Command::new(\"rustc\")", "rustc"),
    ("trybuild::", "trybuild"),
    ("TestCases::new(", "trybuild"),
];
/// Cargo subcommands that resolve the graph without compiling anything.
const QUERY_SUBCOMMANDS: [&str; 4] = ["tree", "metadata", "locate-project", "pkgid"];
/// Flags under which a job (or the gate it runs) executes `#[ignore]`d tests.
const IGNORED_FLAGS: [&str; 3] = ["--run-ignored", "--ignored", "--include-ignored"];

#[derive(Debug, Clone, serde::Serialize)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub detail: String,
}

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub files_scanned: usize,
    /// Every spawn site (expensive-marked and cargo-query).
    pub cargo_call_sites: usize,
    /// The subset marked `// cargo-query:`.
    pub cargo_query_sites: usize,
    /// binary id → names of its `#[ignore = "expensive: …"]` tests.
    pub expensive_tests: BTreeMap<String, Vec<String>>,
    pub findings: Vec<Finding>,
}

/// What `code` spawns (`cargo`/`rustc`/`trybuild`) and the byte offset just past
/// the first trigger, or `None` for a non-spawning, comment or `use` line.
fn spawn_site(code: &str) -> Option<(&'static str, usize)> {
    let trimmed = code.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with("use ") || trimmed.starts_with("pub use ") {
        return None;
    }
    TRIGGERS
        .iter()
        .filter_map(|(pattern, what)| code.find(pattern).map(|at| (at, *what, at + pattern.len())))
        .min_by_key(|(at, ..)| *at)
        .map(|(_, what, end)| (what, end))
}

/// The first string literal in `text` that is not a `-`-flag.
fn first_operand(text: &str) -> Option<&str> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .find(|literal| !literal.starts_with('-'))
}

/// The reason after `prefix` up to the closing quote of an attribute, trimmed.
fn ignore_reason(attr: &str) -> &str {
    attr.strip_prefix(IGNORE)
        .and_then(|rest| rest.split('"').next())
        .unwrap_or_default()
        .trim()
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// The name of a `fn`/`static` item declared on `line`, if any.
fn item_name(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    while let Some(word) = words.next() {
        if word == "fn" || word == "static" {
            let name = words.next()?;
            let end = name
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(name.len());
            return Some(&name[..end]).filter(|name| !name.is_empty());
        }
        if !matches!(word, "pub" | "pub(crate)" | "async" | "unsafe" | "const") {
            return None;
        }
    }
    None
}

/// A test item: its name, declaration line, body range and attributes.
struct Item<'a> {
    name: &'a str,
    decl: usize,
    end: usize,
    attrs: Vec<&'a str>,
}

impl Item<'_> {
    fn is_test(&self) -> bool {
        self.attrs
            .iter()
            .any(|a| *a == "#[test]" || a.starts_with("#[tokio::test"))
    }
    fn is_expensive_ignored(&self) -> bool {
        self.attrs.iter().any(|a| a.starts_with(IGNORE))
    }
}

/// Every `fn`/`static` item in `lines` with its attribute block and body end.
fn items<'a>(lines: &[&'a str]) -> Vec<Item<'a>> {
    let mut out = Vec::new();
    for (decl, line) in lines.iter().enumerate() {
        let Some(name) = item_name(line) else {
            continue;
        };
        let depth = indent(line);
        let mut attrs = Vec::new();
        let mut cursor = decl;
        while cursor > 0 {
            let above = lines[cursor - 1].trim();
            if above.starts_with("#[") || above.starts_with("///") || above.starts_with("//") {
                if above.starts_with("#[") {
                    attrs.push(above);
                }
                cursor -= 1;
            } else {
                break;
            }
        }
        let end = lines[decl..]
            .iter()
            .position(|l| indent(l) == depth && l.trim_end().ends_with(['}', ';']))
            .map_or(lines.len() - 1, |offset| decl + offset);
        out.push(Item {
            name,
            decl,
            end,
            attrs,
        });
    }
    out
}

/// `<package>::<stem>` for a top-level integration-test file
/// (`<member>/tests/<stem>.rs`), else `None`.
pub fn binary_id(root: &Path, rel: &Path) -> Option<String> {
    let tests = rel.parent()?;
    if tests.file_name()? != "tests" {
        return None;
    }
    let stem = rel.file_name()?.to_str()?.strip_suffix(".rs")?;
    let member = tests.parent()?;
    let text = std::fs::read_to_string(root.join(member).join("Cargo.toml")).ok()?;
    let value: toml::Value = toml::from_str(&text).ok()?;
    let package = value.get("package")?.get("name")?.as_str()?;
    Some(format!("{package}::{stem}"))
}

/// The root `tests/` plus every workspace member's `tests/` (a trailing `/*`
/// member glob is expanded), sorted and deduplicated.
fn test_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let text = std::fs::read_to_string(root.join("Cargo.toml"))
        .map_err(|e| format!("cannot read the workspace Cargo.toml: {e}"))?;
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| format!("the workspace Cargo.toml does not parse: {e}"))?;
    let members = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .ok_or("the workspace Cargo.toml has no [workspace] members")?;
    let mut dirs = BTreeSet::from([root.join("tests")]);
    for member in members.iter().filter_map(toml::Value::as_str) {
        match member.strip_suffix("/*") {
            Some(parent) => {
                if let Ok(entries) = std::fs::read_dir(root.join(parent)) {
                    dirs.extend(
                        entries
                            .filter_map(Result::ok)
                            .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
                            .map(|entry| entry.path().join("tests")),
                    );
                }
            }
            None => {
                dirs.insert(root.join(member).join("tests"));
            }
        }
    }
    Ok(dirs.into_iter().collect())
}

/// Every `.rs` file under `dirs` (symlinks and `fixtures/` trees skipped).
fn test_sources(dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = dirs;
    let mut files = Vec::new();
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let path = entry.path();
            if kind.is_dir() {
                if path.file_name().is_some_and(|name| name != "fixtures") {
                    dirs.push(path);
                }
            } else if kind.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// What one file holds: spawn sites, cargo-query sites, expensive-ignored tests.
struct Scan {
    sites: usize,
    queries: usize,
    ignored: Vec<String>,
}

/// Scan one file's lines: spawn sites and `expensive:`-ignored tests.
fn scan_file(rel: &str, text: &str, findings: &mut Vec<Finding>) -> Scan {
    let lines: Vec<&str> = text.lines().collect();
    let items = items(&lines);
    let mut scan = Scan {
        sites: 0,
        queries: 0,
        ignored: Vec::new(),
    };
    let mut finding = |line: usize, detail: String| {
        findings.push(Finding {
            file: rel.to_owned(),
            line: line + 1,
            detail,
        })
    };
    for (index, line) in lines.iter().enumerate() {
        let Some((what, end)) = spawn_site(line) else {
            continue;
        };
        scan.sites += 1;
        let above = index.checked_sub(1).map_or("", |i| lines[i].trim());
        if let Some(reason) = above.strip_prefix(QUERY_MARKER) {
            if reason.trim().is_empty() {
                finding(index, format!("`{QUERY_MARKER}` with an empty reason"));
                continue;
            }
            if what != "cargo" {
                finding(
                    index,
                    format!(
                        "`{QUERY_MARKER}` on a `{what}` spawn — it compiles; mark it `{MARKER} \
                         <reason>` and `{IGNORE} …\"]`"
                    ),
                );
                continue;
            }
            let next = lines.get(index + 1).copied().unwrap_or_default();
            let operand = first_operand(&line[end..]).or_else(|| first_operand(next));
            match operand {
                Some(sub) if QUERY_SUBCOMMANDS.contains(&sub) => scan.queries += 1,
                other => finding(
                    index,
                    format!(
                        "`{QUERY_MARKER}` call runs cargo subcommand {} — only {} compile \
                         nothing; mark it `{MARKER} <reason>` and `{IGNORE} …\"]`",
                        other.map_or("(none on this line or the next)".to_owned(), |s| format!(
                            "`{s}`"
                        )),
                        QUERY_SUBCOMMANDS.join("/")
                    ),
                ),
            }
            continue;
        }
        let Some(reason) = above.strip_prefix(MARKER) else {
            finding(
                index,
                format!(
                    "test spawns {what} without `{MARKER} <reason>` on the line above — build \
                     the binary with `cargo run -p xtask -- build-test-fixtures` (or use \
                     CARGO_BIN_EXE_*), mark it expensive, or (cargo {} only) \
                     `{QUERY_MARKER} <reason>`",
                    QUERY_SUBCOMMANDS.join("/")
                ),
            );
            continue;
        };
        if reason.trim().is_empty() {
            finding(index, format!("`{MARKER}` with an empty reason"));
            continue;
        }
        // The innermost enclosing item.
        let Some(owner) = items
            .iter()
            .filter(|item| item.decl <= index && index <= item.end)
            .max_by_key(|item| item.decl)
        else {
            finding(
                index,
                format!("marked {what} call outside any fn/static item"),
            );
            continue;
        };
        if owner.is_test() {
            if !owner.is_expensive_ignored() {
                finding(
                    index,
                    format!(
                        "test `{}` spawns {what} but is not `{IGNORE} …\"]`",
                        owner.name
                    ),
                );
            }
            continue;
        }
        // Every item that names the owner, transitively (helper -> helper ->
        // test), within this file. Single-hop lookup missed `LazyLock` and
        // `composite_case` wrappers (review 2026-10-23).
        let mut reached: Vec<&Item> = vec![owner];
        loop {
            let before = reached.len();
            for item in items.iter().filter(|item| item.decl != owner.decl) {
                if reached.iter().any(|r| std::ptr::eq(*r, item)) {
                    continue;
                }
                let names_reached = lines[item.decl..=item.end]
                    .iter()
                    .any(|body| reached.iter().any(|r| contains_word(body, r.name)));
                if names_reached {
                    reached.push(item);
                }
            }
            if reached.len() == before {
                break;
            }
        }
        let callers: Vec<&Item> = reached
            .iter()
            .copied()
            .filter(|item| item.is_test() && item.decl != owner.decl)
            .collect();
        if callers.is_empty() {
            finding(
                index,
                format!(
                    "no test reaches `{}` (directly or through helpers) — cannot prove the marked \
                     {what} call is reached only by expensive-ignored tests",
                    owner.name
                ),
            );
        }
        for caller in callers.iter().filter(|c| !c.is_expensive_ignored()) {
            finding(
                caller.decl,
                format!(
                    "test `{}` reaches the marked {what} call via `{}` but is not `{IGNORE} …\"]`",
                    caller.name, owner.name
                ),
            );
        }
    }
    for item in items
        .iter()
        .filter(|item| item.is_test() && item.is_expensive_ignored())
    {
        if item
            .attrs
            .iter()
            .any(|a| a.starts_with(IGNORE) && ignore_reason(a).is_empty())
        {
            finding(
                item.decl,
                format!("test `{}` is `{IGNORE}\"]` with an empty reason", item.name),
            );
        }
        scan.ignored.push(item.name.to_owned());
    }
    scan
}

fn contains_word(haystack: &str, word: &str) -> bool {
    haystack.match_indices(word).any(|(at, _)| {
        let before = haystack[..at].chars().next_back();
        let after = haystack[at + word.len()..].chars().next();
        let boundary = |c: Option<char>| !c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        boundary(before) && boundary(after)
    })
}

/// The gate over a tree rooted at `root` with an explicit register (the test
/// sources and the register; [`audit_jobs`] checks the workflow side).
pub fn audit_with(root: &Path, budgets: &Budgets) -> Report {
    let mut findings = Vec::new();
    let mut cargo_call_sites = 0;
    let mut cargo_query_sites = 0;
    let mut expensive_tests = BTreeMap::new();
    let dirs = test_dirs(root).unwrap_or_else(|detail| {
        findings.push(Finding {
            file: "Cargo.toml".to_owned(),
            line: 0,
            detail,
        });
        Vec::new()
    });
    let files = test_sources(dirs);
    for path in &files {
        let rel_path = path.strip_prefix(root).unwrap_or(path);
        let rel = rel_path.to_string_lossy().into_owned();
        let Ok(text) = std::fs::read_to_string(path) else {
            findings.push(Finding {
                file: rel,
                line: 0,
                detail: "unreadable test source".to_owned(),
            });
            continue;
        };
        let scan = scan_file(&rel, &text, &mut findings);
        cargo_call_sites += scan.sites;
        cargo_query_sites += scan.queries;
        if scan.ignored.is_empty() {
            continue;
        }
        match binary_id(root, rel_path) {
            Some(binary) => {
                if !budgets.expensive.iter().any(|row| row.binary == binary) {
                    findings.push(Finding {
                        file: rel.clone(),
                        line: 0,
                        detail: format!(
                            "{binary} has expensive-ignored tests ({}) but no [[expensive]] row \
                             in {BUDGETS_PATH}",
                            scan.ignored.join(", ")
                        ),
                    });
                }
                expensive_tests.insert(binary, scan.ignored);
            }
            None => findings.push(Finding {
                file: rel,
                line: 0,
                detail: "expensive-ignored tests outside a top-level integration-test file"
                    .to_owned(),
            }),
        }
    }
    for row in &budgets.expensive {
        if !expensive_tests.contains_key(&row.binary) {
            findings.push(Finding {
                file: BUDGETS_PATH.to_owned(),
                line: 0,
                detail: format!(
                    "[[expensive]] {} names no test file with an `{IGNORE}` test — stale row",
                    row.binary
                ),
            });
        }
    }
    Report {
        files_scanned: files.len(),
        cargo_call_sites,
        cargo_query_sites,
        expensive_tests,
        findings,
    }
}

/// The non-comment lines of job `job` in `workflow` (from its `  <job>:` header
/// to the next line at job-key indentation or shallower), or `None`.
pub(crate) fn job_block<'a>(workflow: &'a str, job: &str) -> Option<Vec<&'a str>> {
    let header = format!("  {job}:");
    let mut lines = workflow
        .lines()
        .skip_while(|line| line.trim_end() != header);
    lines.next()?;
    Some(
        lines
            .take_while(|line| line.trim().is_empty() || indent(line) > 2)
            .filter(|line| !line.trim_start().starts_with('#'))
            .collect(),
    )
}

/// Whether a job's lines run ignored tests: an ignored-tests flag on a step, or
/// an `xtask -- <gate>` step whose `xtask/src/<gate>.rs` passes one as a literal.
fn runs_ignored(root: &Path, block: &[&str]) -> bool {
    block.iter().any(|line| {
        IGNORED_FLAGS.iter().any(|flag| line.contains(flag))
            || line.split("xtask -- ").skip(1).any(|rest| {
                let gate = rest.split_whitespace().next().unwrap_or_default();
                let source = root
                    .join("xtask/src")
                    .join(format!("{}.rs", gate.replace('-', "_")));
                !gate.is_empty()
                    && std::fs::read_to_string(source).is_ok_and(|text| {
                        IGNORED_FLAGS
                            .iter()
                            .any(|flag| text.contains(&format!("\"{flag}\"")))
                    })
            })
    })
}

/// Every `[[expensive]]` row's `job` must exist in [`WORKFLOW`] under `root` and
/// run ignored tests — a row naming a job that never runs them registers tests
/// that run nowhere.
pub fn audit_jobs(root: &Path, budgets: &Budgets) -> Vec<Finding> {
    let workflow = match std::fs::read_to_string(root.join(WORKFLOW)) {
        Ok(text) => text,
        Err(e) => {
            return vec![Finding {
                file: WORKFLOW.to_owned(),
                line: 0,
                detail: format!("cannot read the workflow: {e}"),
            }]
        }
    };
    budgets
        .expensive
        .iter()
        .filter_map(|row| {
            let detail = match job_block(&workflow, &row.job) {
                None => format!(
                    "[[expensive]] {} names job `{}`, absent from {WORKFLOW}",
                    row.binary, row.job
                ),
                Some(block) if runs_ignored(root, &block) => return None,
                Some(_) => format!(
                    "[[expensive]] {} names job `{}`, whose steps never run ignored tests ({} \
                     on a step, or an `xtask -- <gate>` whose source passes one) — its \
                     expensive tests would run nowhere",
                    row.binary,
                    row.job,
                    IGNORED_FLAGS.join("/")
                ),
            };
            Some(Finding {
                file: BUDGETS_PATH.to_owned(),
                line: 0,
                detail,
            })
        })
        .collect()
}

/// The nextest filterset for the `expensive-tests` CI job: exactly the
/// registered binaries whose `job` is [`EXPENSIVE_JOB`], restricted to their
/// `expensive:`-ignored tests (run with `--run-ignored ignored-only`).
pub fn expensive_filter(budgets: &Budgets, report: &Report) -> String {
    budgets
        .expensive
        .iter()
        .filter(|row| row.job == EXPENSIVE_JOB)
        .filter_map(|row| {
            let tests = report.expensive_tests.get(&row.binary)?;
            Some(format!(
                "(binary_id({}) & test(/(^|::)({})$/))",
                row.binary,
                tests.join("|")
            ))
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

pub fn run(json: bool, print_filter: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|e| format!("{GATE}: no cwd: {e}"))?;
    let budgets = load_budgets(&root).map_err(|e| format!("{GATE}: {e}"))?;
    let mut report = audit_with(&root, &budgets);
    report.findings.extend(audit_jobs(&root, &budgets));
    let passed = report.findings.is_empty();
    if print_filter {
        if !passed {
            return Err(format!(
                "{GATE}: register is red; fix it before deriving the filter"
            ));
        }
        println!("{}", expensive_filter(&budgets, &report));
        return Ok(());
    }
    if json {
        println!(
            "{}",
            serde_json::json!({
                "gate": GATE,
                "passed": passed,
                "files_scanned": report.files_scanned,
                "cargo_call_sites": report.cargo_call_sites,
                "cargo_query_sites": report.cargo_query_sites,
                "expensive_tests": report.expensive_tests,
                "findings": report.findings,
            })
        );
    }
    if passed {
        if !json {
            println!(
                "{GATE}: PASS ({} test files, {} marked spawn site(s) of which {} cargo-query, {} \
                 expensive binaries)",
                report.files_scanned,
                report.cargo_call_sites,
                report.cargo_query_sites,
                report.expensive_tests.len()
            );
        }
        return Ok(());
    }
    let detail: String = report
        .findings
        .iter()
        .map(|f| format!("- {}:{} — {}\n", f.file, f.line, f.detail))
        .collect();
    let msg = format!(
        "{GATE}: FAIL — {} finding(s):\n{detail}",
        report.findings.len()
    );
    crate::gate_common::emit_command(json, "error", &msg);
    if !json {
        eprintln!("{msg}");
    }
    Err(msg)
}
