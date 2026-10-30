// Story 20-16: local mirrors, not a replacement for the authoritative CI run.
use regex::Regex;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Instant, UNIX_EPOCH};
use syn::visit::Visit;

const WORKFLOW: &str = ".github/workflows/discipline.yml";
// Literal CI invocations; build-test-fixtures precedes every test leg.
const GATES: &[(&str, &str)] = &[
    ("stability-matrix --check", "STABILITY.md"),
    ("error-catalog-check", "docs/errors/error-catalog.json"),
    ("gen-abi-docs --check", "docs-site/abi/v1"),
    ("check-serde-error-handling", "crates"),
    (
        "check-workspace-count",
        "Cargo.toml / docs/kernel-design.md",
    ),
    ("check-manifest-schema-version", "manifest schema"),
    ("check-skill-schema", "skill schema"),
    (
        "check-epic-close-coherence",
        "epic closure / discipline workflow",
    ),
    (
        "check-coverage-matrix-completeness",
        "tests/coverage-matrix.yaml",
    ),
    ("check-test-cost", "xtask/test-budgets.toml"),
    ("check-env-contract", "environment contract"),
    ("check-unsafe", "capability validation sources"),
    ("check-service-boundary", "service boundary sources"),
    ("check-kernel-baseline", "kernel baseline"),
    ("kloc-check", "xtask/kloc.toml"),
    ("check-breaking-md", "BREAKING.md"),
    ("check-composition-root-completeness", "composition roots"),
    ("check-pub-field-constructors", "public field constructors"),
    (
        "check-dependency-closure",
        "Cargo.lock / dependency closure",
    ),
    ("coverage-matrix", "tests/coverage-matrix.yaml"),
    ("check-corpus", "corpus manifest"),
    (
        "abi-diff --base abi-baseline/v1-pre-bump.txt",
        "ABI surface",
    ),
    ("check-empty-kernel", "kernel invariant I9"),
    ("check-loom", "loom substrate"),
    ("check-fr47", "FR47 bindings"),
    ("check-a2a-sender-completeness", "A2A sender boundary"),
    ("check-judge-config", "pinned judge configuration"),
    ("check-security-md", "SECURITY.md"),
    ("templates-regen --check", "example Spirit templates"),
    ("corpus-staleness", "corpus validity dates"),
    (
        "calibrate --corpus calibration-seed-v0.1 --n 100 --p 0.95",
        "calibration seed",
    ),
    ("check-review-findings-resolved", "review closure"),
    ("check-dev-record-completeness", "story records"),
    ("check-dev-model-tier", "story model tiers"),
    ("check-ship-gate-completeness", "ship registry"),
    ("check-pentest-gate", "pen-test evidence"),
    ("check-third-party-trial", "trial evidence"),
    ("check-trial-attestation", "trial attestation"),
    ("check-red-team-gate", "red-team evidence"),
    ("check-migration-merkle", "migration evidence"),
    ("nfr-onb-1-gate --check", "onboarding evidence"),
    ("cassette-age-gate", "cassette age policy"),
    ("build-test-fixtures", "target/maos-fixtures/manifest.json"),
];

#[derive(Debug, Serialize)]
struct Record {
    check: String,
    artifact: String,
    status: &'static str,
    detail: String,
    wall_ms: u128,
}

/// A SKIP line a leg printed: visible and countable, but only what that leg's
/// output said, never an independently established check disposition.
#[derive(Debug, Serialize)]
struct OutputDiagnostic {
    check: String,
    artifact: String,
    line: String,
}

#[derive(Default, Serialize)]
struct Report {
    base: Option<String>,
    base_age_seconds: Option<u64>,
    changed_paths: BTreeSet<String>,
    records: Vec<Record>,
    output_diagnostics: Vec<OutputDiagnostic>,
    wall_ms: u128,
}

impl Report {
    fn add(
        &mut self,
        check: &str,
        artifact: &str,
        status: &'static str,
        detail: impl Into<String>,
    ) {
        self.records.push(Record {
            check: check.into(),
            artifact: artifact.into(),
            status,
            detail: detail.into(),
            wall_ms: 0,
        });
    }

    fn exit_code(&self) -> i32 {
        i32::from(self.records.iter().any(|r| r.status == "red"))
    }

    fn result<T>(&mut self, check: &str, artifact: &str, result: Result<T, String>) -> Option<T> {
        let (value, status, detail) = match result {
            Ok(value) => (Some(value), "pass", "exit 0".into()),
            Err(error) => (None, "red", error),
        };
        self.add(check, artifact, status, detail);
        value
    }

    fn command(&mut self, check: &str, artifact: &str, command: &mut Command) -> Option<Output> {
        let started = Instant::now();
        let output = execute(command);
        let judged = output.as_ref().map_err(Clone::clone).and_then(success);
        self.result(check, artifact, judged);
        self.records.last_mut().unwrap().wall_ms = started.elapsed().as_millis();
        let output = output.ok()?;
        for line in combined(&output)
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("SKIP:") || l.starts_with("SKIP "))
        {
            self.output_diagnostics.push(OutputDiagnostic {
                check: check.into(),
                artifact: artifact.into(),
                line: line.into(),
            });
        }
        Some(output)
    }

    fn text(&self) -> String {
        let mut lines = Vec::new();
        if let Some(base) = &self.base {
            lines.push(format!(
                "BASE {base} — age {}s — {} changed paths",
                self.base_age_seconds.unwrap_or(0),
                self.changed_paths.len()
            ));
        }
        for record in &self.records {
            let label = match record.status {
                "red" => "RED",
                "skip" => "SKIP",
                "advisory" => "ADVISORY",
                _ => continue,
            };
            lines.push(format!(
                "{label} {} — {} — {}",
                record.check,
                record.artifact,
                single_line(&record.detail)
            ));
        }
        for diagnostic in &self.output_diagnostics {
            lines.push(format!(
                "DIAGNOSTIC {} — {} — captured output, not a check disposition: {}",
                diagnostic.check, diagnostic.artifact, diagnostic.line
            ));
        }
        lines.push(format!("prepush: {} checks run, {} skipped-with-reason, {} red, {} captured SKIP output line(s) (diagnostic only), wall {:.3}s; landing CI remains authoritative",
            self.records.iter().filter(|r| r.status != "skip").count(),
            self.records.iter().filter(|r| r.status == "skip").count(),
            self.records.iter().filter(|r| r.status == "red").count(),
            self.output_diagnostics.len(), self.wall_ms as f64 / 1000.0));
        lines.join("\n")
    }
}

fn single_line(text: &str) -> String {
    let mut words = text.split_whitespace();
    let first = words.next().unwrap_or_default().to_owned();
    words.fold(first, |line, word| line + " " + word)
}
fn combined(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stdout).into_owned() + "\n";
    text + &String::from_utf8_lossy(&output.stderr)
}
fn command(root: &Path, tool: impl AsRef<std::ffi::OsStr>, args: &str) -> Command {
    let mut cmd = Command::new(tool);
    cmd.current_dir(root).args(args.split_ascii_whitespace());
    cmd
}
fn execute(cmd: &mut Command) -> Result<Output, String> {
    cmd.env_remove("CI")
        .env_remove("GITHUB_ACTIONS")
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .map_err(|e| format!("required tool {:?}: {e}", cmd.get_program()))
}
fn success(output: &Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    // Bounded RED detail (workspace nextest echoes every passing test): 40 lines of
    // 400 chars. Rank nextest/libtest failure status > panic > `error`, and lift the
    // final 20 to at least `error`, so captured `error` text never outranks the failing
    // test; the highest 40 (latest wins within a rank) show in output order, so an
    // `error:` header precedes its `-->` location and a panic stays beside its FAIL.
    let failing =
        Regex::new(r"^\s*([A-Z-]*FAIL|TIMEOUT|ABORT|SIG[A-Z]+) \[|^test .*[.:] FAILED").unwrap();
    let text = combined(output);
    let lines: Vec<_> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let rank = |l: &str| u8::from(failing.is_match(l)) * 2 + u8::from(l.contains("panicked at"));
    let rank = |l: &str| rank(l) * 2 + u8::from(l.contains("error"));
    let mut order: Vec<_> = (0..lines.len()).collect();
    order.sort_by_cached_key(|&i| (rank(lines[i]).max(u8::from(i + 20 >= lines.len())), i));
    let order: BTreeSet<_> = order.drain(lines.len().saturating_sub(40)..).collect();
    let cut = |i: usize| lines[i].chars().take(400).collect::<String>();
    let kept: Vec<_> = order.iter().map(|&i| cut(i)).collect();
    let (n, shown) = (lines.len() - kept.len(), single_line(&kept.join("\n")));
    let (status, text) = (output.status, format!("{n} lines omitted; {shown}"));
    Err(format!("expected exit 0, actual {status}; {text}"))
}
fn capture(root: &Path, tool: &str, args: &str) -> Result<String, String> {
    capture_command(&mut command(root, tool, args))
}
fn capture_command(cmd: &mut Command) -> Result<String, String> {
    let output = execute(cmd)?;
    success(&output)?;
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

fn changes(root: &Path, report: &mut Report) -> Result<(), String> {
    let mut base = capture(root, "git", "merge-base origin/main HEAD")?;
    base.truncate(base.trim_end().len());
    let timestamp: u64 = capture_command(command(root, "git", "show -s --format=%ct").arg(&base))?
        .trim()
        .parse()
        .map_err(|e| format!("base timestamp: {e}"))?;
    let now = UNIX_EPOCH.elapsed().map_err(|e| e.to_string())?.as_secs();
    // Union, not net base→working: HEAD may change a path the working tree
    // restores. --no-renames keeps both endpoints of every rename.
    let mut paths = BTreeSet::new();
    for listed in [
        capture_command(
            command(root, "git", "diff --name-only -z --no-renames")
                .arg(&base)
                .args(["HEAD", "--"]),
        )?,
        capture(root, "git", "diff --name-only -z --no-renames HEAD --")?,
        capture(root, "git", "ls-files --others --exclude-standard -z")?,
    ] {
        paths.extend(listed.split_terminator('\0').map(str::to_owned));
    }
    report.changed_paths = paths;
    report.base_age_seconds = Some(now.saturating_sub(timestamp));
    report.base = Some(base);
    Ok(())
}

// --no-deps is sufficient: every locally editable dependency is in packages;
// path dependencies are followed transitively, including renamed dependencies.
fn binary_paths(metadata: &Value) -> Result<BTreeSet<String>, String> {
    let packages = metadata["packages"]
        .as_array()
        .ok_or("metadata lacks packages")?;
    let root = metadata["workspace_root"]
        .as_str()
        .ok_or("metadata lacks workspace_root")?;
    let mut pending = vec!["maos-bin"];
    let mut paths = BTreeSet::new();
    while let Some(name) = pending.pop() {
        let package = packages
            .iter()
            .find(|p| p["name"].as_str() == Some(name))
            .ok_or_else(|| format!("metadata missing {name}"))?;
        let manifest = package["manifest_path"].as_str().map(Path::new);
        let directory = manifest
            .and_then(Path::parent)
            .ok_or("package lacks manifest_path")?;
        let directory = directory.strip_prefix(root).map_err(|e| e.to_string())?;
        if !paths.insert(format!("{}/", directory.display())) {
            continue;
        }
        for dep in package["dependencies"]
            .as_array()
            .ok_or("package lacks dependencies")?
        {
            if dep["path"].is_string() && dep["kind"].as_str() != Some("dev") {
                pending.push(dep["name"].as_str().ok_or("dependency lacks name")?);
            }
        }
    }
    Ok(paths)
}
fn size_trigger(paths: &BTreeSet<String>, closure: &BTreeSet<String>) -> bool {
    paths.iter().any(|p| {
        p == "Cargo.toml" || p == "Cargo.lock" || closure.iter().any(|prefix| p.starts_with(prefix))
    })
}
fn docs_trigger(paths: &BTreeSet<String>) -> bool {
    paths.iter().any(|p| {
        p.starts_with("docs-site/")
            || p.starts_with("docs/errors/")
            || p == "xtask/error-catalog.toml"
    })
}

pub(crate) fn run(json: bool) -> ! {
    let started = Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut report = Report::default();
    let original = tracked_tree(root);
    let changed = changes(root, &mut report);
    report.result(
        "changed-paths",
        "merge-base..HEAD / HEAD..working tree / untracked",
        changed,
    );
    let metadata = capture(
        root,
        "cargo",
        "metadata --locked --no-deps --format-version 1 --color never",
    )
    .and_then(|s| serde_json::from_str::<Value>(&s).map_err(|e| e.to_string()));
    let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"));
    for &(invocation, artifact) in GATES {
        let name = invocation.split(' ').next().unwrap();
        let json = (!matches!(name, "gen-abi-docs" | "build-test-fixtures")).then_some("--json");
        match &exe {
            Ok(exe) => {
                report.command(name, artifact, command(root, exe, invocation).args(json));
            }
            Err(e) => report.add(name, artifact, "red", e),
        }
    }
    let closure = match (&metadata, &report.base) {
        (Err(error), _) => Err(error.clone()),
        (_, None) => Err("change detection unavailable; cannot decide trigger".into()),
        (Ok(metadata), Some(_)) => binary_paths(metadata),
    };
    if let Some(closure) = report.result(
        "binary-size:trigger",
        "cargo metadata / change set",
        closure,
    ) {
        binary_size(root, &mut report, &closure);
    }
    let docs_skip = (report.base.is_some() && !docs_trigger(&report.changed_paths))
        .then_some("no docs-site/**, docs/errors/** or xtask/error-catalog.toml changes");
    let deny =
        report.changed_paths.contains("Cargo.lock") || report.changed_paths.contains("deny.toml");
    let deny_skip =
        (report.base.is_some() && !deny).then_some("neither Cargo.lock nor deny.toml changed");
    let mut junit = metadata
        .as_ref()
        .ok()
        .and_then(|m| m["target_directory"].as_str())
        .map(|target| Path::new(target).join("nextest/default/junit.xml"));
    let mut docs_built = false;
    let docs_root = root.join("docs-site");
    // Literal argv only: dynamic paths are always passed with Command::arg.
    // Fixtures above precede the unconditional nextest/libtest/doctest legs.
    for (name, artifact, invocation) in [
        ("fmt", "workspace Rust sources", "cargo fmt --all -- --check"),
        ("docs:troubleshoot-bidi", "docs-site/route-manifest.json / docs/errors/error-catalog.json",
            "node scripts/gate-troubleshoot-bidi.js"),
        ("docs:glossary-lock", "docs-site/i18n/ko / glossary", "node scripts/gate-glossary-lock.js"),
        ("docs:anchor-ids", "docs-site anchors", "node scripts/gate-anchor-ids.js"),
        ("docs:ko-coverage", "docs-site/i18n/ko (KO_COVERAGE_MIN=100)", "node scripts/gate-ko-coverage.js"),
        ("docs:build", "docs-site/build", "npm run build"),
        ("docs:routes", "docs-site/build / route-manifest.json", "node scripts/gate-routes.js"),
        ("cargo-deny", "Cargo.lock / deny.toml (required cargo-deny)", "cargo deny check"),
        ("workspace-nextest", "workspace tests / cross-crate enrolment scanners",
            "cargo nextest run --workspace --locked --color never --no-fail-fast --success-output immediate --failure-output final"),
        ("kernel-core-lib", "maos-kernel-core lib tests",
            "cargo test -p maos-kernel-core --lib --locked --color never -- --nocapture"),
        ("workspace-doc", "workspace doctests", "cargo test --workspace --doc --locked --color never"),
    ] {
        let skip = match name {
            "docs:build" | "docs:routes" => docs_skip,
            "cargo-deny" => deny_skip,
            _ => None,
        };
        if let Some(reason) = skip {
            report.add(name, artifact, "skip", reason);
            continue;
        }
        if name == "docs:routes" && !docs_built {
            report.add(name, artifact, "red", "fresh docs build unavailable; refusing stale build route proof");
            continue;
        }
        if name == "workspace-nextest" {
            // Removing the old report makes freshness independent of bytes/mtime.
            junit = junit.filter(|path| fs::remove_file(path).is_ok() || !path.exists());
        }
        let directory: &Path = if name.starts_with("docs:") { &docs_root } else { root };
        let (tool, args) = invocation.split_once(' ').unwrap();
        let mut cmd = command(directory, tool, args);
        if name.starts_with("docs:") {
            cmd.env("LOCALE", "ko").env("KO_COVERAGE_MIN", "100");
        }
        let output = report.command(name, artifact, &mut cmd);
        if name == "docs:build" {
            docs_built = output.is_some_and(|o| o.status.success());
        }
    }
    match (&exe, &junit) {
        (Ok(exe), Some(path)) if path.exists() => test_budget(&mut report, &path.display().to_string(),
            command(root, exe, "check-test-budget --json --junit").arg(path)),
        _ => report.add("test-budget", "nextest/default/junit.xml", "skip",
            "no fresh local nextest junit; timing judgment is advisory and stale junit is never read"),
    }
    let pins = fs::read_to_string(root.join(WORKFLOW))
        .map_err(|e| e.to_string())
        .and_then(|workflow| read_pins(&workflow));
    if let Some(pins) = report.result("ci-pins", WORKFLOW, pins) {
        check_pins(root, &pins, &mut report);
    }
    let preserved = original.and_then(|before| match before == tracked_tree(root)? {
        true => Ok(()),
        false => Err("a leg modified tracked files; caller's original tree was not preserved (no destructive reset attempted)".into()),
    });
    report.result("tracked-tree", "git diff HEAD / tracked status", preserved);
    report.wall_ms = started.elapsed().as_millis();
    match json
        .then(|| serde_json::to_string_pretty(&report))
        .unwrap_or_else(|| Ok(report.text()))
    {
        Ok(text) => println!("{text}"),
        Err(error) => {
            eprintln!("RED report — JSON — {error}");
            std::process::exit(1);
        }
    }
    std::process::exit(report.exit_code());
}

fn tracked_tree(root: &Path) -> Result<(String, String), String> {
    Ok((
        capture(root, "git", "diff --binary HEAD --")?,
        capture(root, "git", "status --porcelain=v1 -z --untracked-files=no")?,
    ))
}

/// Only a timing judgment is advisory. Both check-test-budget timing findings
/// (per-binary and suite total) carry "(Σ per-test time)"; a stale
/// `[[exception]]` row, any other finding or a tool/execution error stays red.
fn test_budget(report: &mut Report, junit: &str, budget: &mut Command) {
    let output = report.command("test-budget", junit, budget);
    let stdout = output.map(|o| o.stdout).unwrap_or_default();
    let timing = |f: &Value| f.as_str().unwrap_or("").contains("(Σ per-test time)");
    let judged = stdout.split(|&b| b == b'\n').any(|line| {
        let v = serde_json::from_slice::<Value>(line).unwrap_or_default();
        let findings = v["findings"].as_array().filter(|f| !f.is_empty());
        v["gate"] == "check-test-budget" && findings.is_some_and(|f| f.iter().all(timing))
    });
    let record = report.records.last_mut().unwrap();
    if judged && record.status == "red" {
        record.status = "advisory";
    }
}

fn binary_size(root: &Path, report: &mut Report, closure: &BTreeSet<String>) {
    if !size_trigger(&report.changed_paths, closure) {
        return report.add(
            "binary-size",
            "release maos <= 33554432 B",
            "skip",
            "no Cargo.toml, Cargo.lock, maos-bin or transitive local dependency changes",
        );
    }
    let mut build = command(
        root,
        "cargo",
        "build --locked --release -p maos-bin --bin maos --message-format=json --color never",
    );
    let output = report.command("binary-size:build", "release maos", &mut build);
    let Some(output) = output.filter(|o| o.status.success()) else {
        return;
    };
    let executable = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
        .filter(|v| v["target"]["name"] == "maos")
        .find_map(|v| v["executable"].as_str().map(str::to_owned));
    let result = (|| -> Result<(), String> {
        let source = executable.ok_or("cargo did not report the maos executable")?;
        let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
        let stripped = temp.path().join("maos");
        capture_command(command(root, "strip", "-o").arg(&stripped).arg(source))?;
        let actual = fs::metadata(stripped).map_err(|e| e.to_string())?.len();
        if actual > 33_554_432 {
            return Err(format!("expected <= 33554432 B, actual {actual} B"));
        }
        Ok(())
    })();
    report.result(
        "binary-size",
        "tests/integration/onb_nfr2_timing.sh: stripped release maos",
        result,
    );
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Selection {
    args: Vec<String>,
    ignored: bool,
    include_ignored: bool,
    nextest: bool,
    binary_ids: BTreeSet<String>,
}
#[derive(Debug, Default)]
struct Pin {
    location: String,
    selections: Vec<Selection>,
    counts: Vec<usize>,
    markers: Vec<(String, usize)>,
    ansi_safe: bool,
    parse_error: Option<String>,
}
fn yaml_env(value: &serde_yaml::Value, env: &mut BTreeMap<String, String>) {
    for (key, val) in value.as_mapping().into_iter().flatten() {
        let number = val.as_i64().map(|n| n.to_string());
        let val = val.as_str().map(str::to_owned).or(number);
        if let (Some(key), Some(val)) = (key.as_str(), val) {
            env.insert(key.into(), val);
        }
    }
}
fn expand(text: &str, env: &BTreeMap<String, String>) -> String {
    let variable = Regex::new(r"\$\{\{ env\.(\w+) \}\}|\$\{(\w+)\}|\$(\w+)").unwrap();
    variable
        .replace_all(text, |captures: &regex::Captures<'_>| {
            let key = captures.iter().skip(1).flatten().next().unwrap().as_str();
            env.get(key)
                .map_or(&text[captures.get(0).unwrap().range()], String::as_str)
        })
        .into_owned()
}
// This is a reader, never a shell evaluator: unknown selection syntax fails
// closed instead of dropping a filter or executing arbitrary workflow shell.
fn selection(text: &str) -> Result<Selection, String> {
    let words = Regex::new(r#"'[^']*'|"[^"]*"|[^\s]+"#).unwrap();
    let mut selected = Selection::default();
    let mut tokens = words
        .find_iter(text)
        .map(|m| m.as_str().trim_matches(['\'', '"']));
    let mut harness = false;
    while let Some(token) = tokens.next() {
        if token.starts_with("2>") || token.starts_with('|') || token.starts_with(';') {
            break;
        }
        match token {
            "--" => {
                harness = true;
                selected.args.push("--".into());
            }
            "--ignored" => selected.ignored = true,
            "--include-ignored" => selected.include_ignored = true,
            "--nocapture" | "--no-capture" | "--list" | "--extract-overwrite" => {}
            t if t.starts_with("--test-threads=") => {}
            "--run-ignored" => match tokens.next() {
                Some("only" | "ignored-only") => selected.ignored = true,
                Some("all") => selected.include_ignored = true,
                Some("default") => {}
                _ => return Err("unresolved --run-ignored mode".into()),
            },
            "-p" | "--test" | "--features" | "--manifest-path" | "-E" | "--skip" => {
                let value = tokens
                    .next()
                    .ok_or_else(|| format!("missing value for {token}"))?;
                if value.contains('$') {
                    return Err(format!("unresolved selection {token} {value}"));
                }
                selected.args.extend([token.into(), value.into()]);
            }
            "--test-threads" | "--archive-file" | "--extract-to" | "--workspace-remap"
            | "--profile" | "--color" => {
                tokens
                    .next()
                    .ok_or_else(|| format!("missing value for {token}"))?;
            }
            "--locked" | "--no-fail-fast" => {}
            "--lib" | "--workspace" | "--no-default-features" | "--release" | "--exact" => {
                selected.args.push(token.into())
            }
            t if t.starts_with('-') => return Err(format!("unsupported selection option {t}")),
            t if !t.is_empty() => {
                if t.contains('$') || t.contains([';', '|']) {
                    return Err(format!("unresolved selection {t}"));
                }
                // libtest's trailing substring filter is also nextest's filter.
                if harness || !t.starts_with('>') {
                    selected.args.push(t.into());
                }
            }
            _ => {}
        }
    }
    if selected.ignored && selected.include_ignored {
        return Err("conflicting ignored-test selections".into());
    }
    Ok(selected)
}

fn bind_archive(
    raw: &str,
    env: &BTreeMap<String, String>,
    nextest: bool,
    families: &BTreeMap<String, Selection>,
) -> Result<Selection, String> {
    let mut selected = selection(raw)?;
    selected.nextest = nextest;
    if !raw.contains("--archive-file") {
        return Ok(selected);
    }
    let field = |name: &str| {
        env.get(name)
            .ok_or_else(|| format!("archive selection lacks {name}"))
    };
    let packages: BTreeSet<_> = field("NEXTEST_ARCHIVE_PACKAGES")?
        .split_whitespace()
        .collect();
    let features = field("NEXTEST_ARCHIVE_FEATURES")?;
    let tests: BTreeSet<_> = field("NEXTEST_ARCHIVE_TESTS")?.split_whitespace().collect();
    if packages.is_empty() || tests.is_empty() || features.contains(['$', ' ']) {
        return Err("pinned archive requires literal package/features/test metadata".into());
    }
    let features: BTreeSet<_> = features.split(',').filter(|s| !s.is_empty()).collect();
    let path = archive_path(raw)?;
    let producer = families
        .get(&path)
        .ok_or_else(|| format!("no archive producer matches {path}"))?;
    if packages != option_values(&producer.args, "-p")
        || features != option_values(&producer.args, "--features")
    {
        return Err(format!(
            "{path}: archive metadata differs from producer selection"
        ));
    }
    let available = option_values(&producer.args, "--test");
    if !available.is_empty() && !tests.is_subset(&available) {
        return Err(format!(
            "{path}: metadata test target is not in producer archive"
        ));
    }
    if selected.args.iter().any(|arg| {
        arg.starts_with('-') && !matches!(arg.as_str(), "-E" | "--skip" | "--exact" | "--")
    }) {
        return Err("archive runtime carries build selectors; use archive metadata".into());
    }
    let filters: Vec<_> = selected.args.windows(2).filter(|w| w[0] == "-E").collect();
    let [filter] = filters[..] else {
        return Err("archive pin requires one literal binary_id filter".into());
    };
    let expression = &filter[1];
    let identity = Regex::new(r"^binary_id\(=?(([A-Za-z0-9_-]+)::([A-Za-z0-9_-]+))\)$").unwrap();
    for term in expression.split('|') {
        let found = identity
            .captures(term.trim())
            .ok_or_else(|| format!("unresolved archive binary_id filter {expression:?}"))?;
        let id = &found[1];
        if !packages.contains(&found[2]) || !tests.contains(&found[3]) {
            return Err(format!("binary_id({id}) is outside archive metadata"));
        }
        selected.binary_ids.insert(id.to_owned());
    }
    // The exact runtime filter and listed_counts exclude producer-only targets.
    let mut args = producer.args.clone();
    args.append(&mut selected.args);
    selected.args = args;
    Ok(selected)
}

fn option_values<'a>(args: &'a [String], name: &str) -> BTreeSet<&'a str> {
    args.windows(2)
        .filter(|w| w[0] == name)
        .flat_map(|w| w[1].split(','))
        .collect()
}

fn archive_path(raw: &str) -> Result<String, String> {
    Regex::new(r#"--archive-file(?:=|\s+)["']?([^"'\s]+)"#)
        .unwrap()
        .captures(raw)
        .map(|c| c[1].to_owned())
        .ok_or_else(|| "archive command lacks literal archive path".into())
}

fn read_pins(workflow: &str) -> Result<Vec<Pin>, String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(workflow).map_err(|e| e.to_string())?;
    let jobs = yaml["jobs"].as_mapping().ok_or("workflow lacks jobs")?;
    let cargo =
        Regex::new(r"(cargo\s+test|(?:cargo|cargo-nextest)\s+nextest\s+run)\s+([^\n]+)").unwrap();
    let libtest = Regex::new(r"test result: ok\\?\. (\d+) passed").unwrap();
    let nextest = Regex::new(r"(\d+) tests run: (\d+) passed").unwrap();
    let expected = Regex::new(r#"--expected-count\s+["']?(\d+)(?:["']?(?:\s|$))"#).unwrap();
    let marker = Regex::new(r"'([A-Z][A-Z0-9-]* ):(\d+)'").unwrap();
    let continuation = Regex::new(r"\\\n\s*").unwrap();
    let color = Regex::new(r#"(?:^|[ ;])(?:export\s+)?CARGO_TERM_COLOR=['"]?([A-Za-z]+)"#).unwrap();
    let flat = continuation.replace_all(workflow, " ");
    let archive = Regex::new(r"cargo\s+nextest\s+archive\s+([^\n]+)").unwrap();
    let mut families = BTreeMap::new();
    for found in archive.captures_iter(&flat) {
        let path = archive_path(&found[1])?;
        let selected = selection(&found[1])?;
        if families
            .insert(path.clone(), selected.clone())
            .is_some_and(|old| old != selected)
        {
            return Err(format!(
                "conflicting archive producer selections for {path}"
            ));
        }
    }
    let mut pins = Vec::new();
    for (job_name, job) in jobs {
        let job_name = job_name.as_str().ok_or("non-string job name")?;
        let number = |value: &str| {
            value
                .parse::<usize>()
                .map_err(|e| format!("{job_name}: invalid count {value:?}: {e}"))
        };
        let mut env = BTreeMap::new();
        yaml_env(&yaml["env"], &mut env);
        yaml_env(&job["env"], &mut env);
        for step in job["steps"].as_sequence().into_iter().flatten() {
            let Some(run) = step["run"].as_str() else {
                continue;
            };
            let mut step_env = env.clone();
            yaml_env(&step["env"], &mut step_env);
            let run = continuation
                .replace_all(&expand(run, &step_env), " ")
                .into_owned();
            let empty_pin = || Pin {
                location: format!("{job_name}/{}", step["name"].as_str().unwrap_or("run")),
                ansi_safe: true,
                ..Pin::default()
            };
            let mut current = empty_pin();
            let mut pinned = false;
            let mut raw_commands = Vec::new();
            let mut color_env = step_env
                .get("CARGO_TERM_COLOR")
                .is_some_and(|v| v == "never");
            let mut guarded = false;
            let mut step_pins = Vec::new();
            for line in run.lines().filter(|l| !l.trim_start().starts_with('#')) {
                let found = cargo
                    .captures(line)
                    .filter(|c| !line[..c.get(0).unwrap().start()].contains("echo "));
                if let Some(found) = found {
                    if pinned {
                        step_pins.push(std::mem::replace(&mut current, empty_pin()));
                        pinned = false;
                        raw_commands.clear();
                    }
                    let prefix = &line[..found.get(0).unwrap().start()];
                    let inline = color.captures_iter(prefix).last();
                    current.ansi_safe &= ansi_safe(
                        &found[2],
                        inline.map_or(color_env, |capture| &capture[1] == "never"),
                    );
                    raw_commands
                        .push((found.get(2).unwrap().as_str(), found[1].contains("nextest")));
                } else if line.trim_start().starts_with("export CARGO_TERM_COLOR=") {
                    color_env = color
                        .captures(line)
                        .is_some_and(|value| &value[1] == "never")
                        && !line.contains([';', '|']);
                } else if line.trim() == "unset CARGO_TERM_COLOR" {
                    color_env = false;
                }
                guarded |= !raw_commands.is_empty()
                    && (line.contains("grep ")
                        || line.contains("sed ")
                        || line.contains("check-log"));
                let mut counts: Vec<_> = expected
                    .captures_iter(line)
                    .map(|c| number(&c[1]))
                    .collect::<Result<_, _>>()?;
                let unresolved = line.matches("--expected-count").count() != counts.len();
                let summary = nextest.captures(line);
                if counts.is_empty() {
                    if let Some(c) = libtest.captures(line).or_else(|| nextest.captures(line)) {
                        counts.push(number(&c[1])?);
                    }
                }
                let markers: Vec<_> = marker
                    .captures_iter(line)
                    .map(|c| number(&c[2]).map(|n| (c[1].to_owned(), n)))
                    .collect::<Result<_, _>>()?;
                if counts.is_empty() && markers.is_empty() && !unresolved {
                    continue;
                }
                // Diagnostic greps need ANSI safety, not a named test binding.
                // Only an actual count/marker pin invokes the strict grammar.
                if !pinned {
                    for &(raw, nextest) in &raw_commands {
                        match bind_archive(raw, &step_env, nextest, &families) {
                            Ok(selected) => current.selections.push(selected),
                            Err(error) => current.parse_error = Some(error),
                        }
                    }
                    pinned = true;
                }
                if raw_commands.is_empty() {
                    current.parse_error = Some("pin has no cargo test/nextest selection".into());
                    current.ansi_safe = false;
                }
                if unresolved {
                    current.parse_error = Some(format!("unresolved count pin: {line}"));
                }
                if summary.is_some_and(|c| c[1] != c[2]) {
                    current.parse_error =
                        Some(format!("summary pin permits non-passing tests: {line}"));
                }
                if !counts.is_empty() {
                    if !current.counts.is_empty() && current.counts != counts {
                        current.parse_error = Some(format!(
                            "conflicting count guards {:?} and {counts:?}",
                            current.counts
                        ));
                    } else {
                        current.counts = counts;
                    }
                }
                current.markers.extend(markers);
            }
            if pinned || guarded {
                step_pins.push(current);
            }
            let bound: BTreeSet<_> = step_pins
                .iter()
                .flat_map(|p| &p.selections)
                .flat_map(|s| &s.binary_ids)
                .filter_map(|id| id.split_once("::").map(|(_, test)| test))
                .collect();
            let declared: BTreeSet<_> = step_env
                .get("NEXTEST_ARCHIVE_TESTS")
                .map_or("", String::as_str)
                .split_whitespace()
                .collect();
            if !bound.is_empty() && declared != bound {
                let error = format!(
                    "archive test metadata {declared:?} != runtime binary_id bindings {bound:?}"
                );
                for pin in &mut step_pins {
                    pin.parse_error = Some(error.clone());
                }
            }
            pins.extend(step_pins);
        }
    }
    if !pins
        .iter()
        .any(|p| !p.counts.is_empty() || !p.markers.is_empty() || p.parse_error.is_some())
    {
        return Err("no CI count or marker pins found; refusing empty reader".into());
    }
    Ok(pins)
}

fn ansi_safe(raw: &str, mut safe: bool) -> bool {
    let mut words = raw.split_whitespace();
    while let Some(word) = words.next() {
        if word.starts_with('|') || word.starts_with(';') || word == "--" {
            break;
        }
        if word == "--color" {
            safe = words.next() == Some("never");
        } else if let Some(value) = word.strip_prefix("--color=") {
            safe = value == "never";
        }
    }
    safe
}

fn listed_counts(json: &str, selection: &Selection) -> Result<Vec<usize>, String> {
    let value: Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let suites = value["rust-suites"]
        .as_object()
        .ok_or("nextest list lacks rust-suites")?;
    let mut counts = BTreeMap::new();
    let mut matched = BTreeSet::new();
    for (id, suite) in suites {
        let cases = suite["testcases"]
            .as_object()
            .ok_or("nextest suite lacks testcases")?;
        let mut count = 0;
        let mut suite_matches = false;
        for case in cases.values() {
            let ignored = case["ignored"].as_bool();
            let status = case["filter-match"]["status"].as_str();
            let (Some(ignored), Some(status @ ("matches" | "mismatch"))) = (ignored, status) else {
                return Err(format!("nextest suite {id} lacks filter/ignored metadata"));
            };
            if status == "matches" {
                suite_matches = true;
                count += usize::from(selection.include_ignored || ignored == selection.ignored);
            }
        }
        if suite_matches {
            matched.insert(id.clone());
        }
        // nextest may include binaries completely excluded by a selection.
        if count > 0 {
            counts.insert(id, count);
        }
    }
    if !selection.binary_ids.is_empty() && matched != selection.binary_ids {
        return Err(format!(
            "expected exact binary_id bindings {:?}, actual selected suites {matched:?}",
            selection.binary_ids
        ));
    }
    Ok(counts.into_values().collect())
}

fn check_pins(root: &Path, pins: &[Pin], report: &mut Report) {
    let mut cache: BTreeMap<Selection, Result<Vec<usize>, String>> = BTreeMap::new();
    for pin in pins {
        let artifact = format!("{WORKFLOW}:{}", pin.location);
        if !pin.ansi_safe {
            report.add("ci-pins:ansi", &artifact, "red", "cargo output guard requires --color never or CARGO_TERM_COLOR=never (workflow default may be always)");
        }
        if let Some(error) = &pin.parse_error {
            report.add("ci-pins:selection", &artifact, "red", error);
            continue;
        }
        if !pin.counts.is_empty() {
            let actual = ordered_counts(&pin.selections, |selected| {
                cache
                    .entry(selected.clone())
                    .or_insert_with(|| {
                        let json = capture_command(command(root, "cargo",
                            "nextest list --locked --color never --message-format json --ignore-default-filter --run-ignored all")
                            .args(&selected.args))?;
                        listed_counts(&json, selected)
                    })
                    .clone()
            });
            let judged = actual.and_then(|actual| match actual == pin.counts {
                true => Ok(()),
                false => Err(format!(
                    "expected {:?}, actual {actual:?}; selections {:?}",
                    pin.counts, pin.selections
                )),
            });
            report.result("ci-pins:count", &artifact, judged);
        }
        if !pin.markers.is_empty() {
            marker_pins(root, pin, &artifact, report);
        }
    }
}

fn ordered_counts(
    selections: &[Selection],
    mut list: impl FnMut(&Selection) -> Result<Vec<usize>, String>,
) -> Result<Vec<usize>, String> {
    let mut ordered = Vec::new();
    for selection in selections {
        let counts = list(selection)?;
        if selection.nextest {
            ordered.push(counts.iter().sum());
        } else {
            ordered.extend(counts);
        }
    }
    Ok(ordered)
}

#[derive(Default)]
struct Emissions {
    sites: Vec<(String, bool)>,
    dynamic: usize,
    test_function: bool,
}
impl<'ast> Visit<'ast> for Emissions {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let previous = self.test_function;
        self.test_function = node
            .attrs
            .iter()
            .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"));
        syn::visit::visit_item_fn(self, node);
        self.test_function = previous;
    }
    // Loop and closure bodies run an indeterminate number of times.
    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        use syn::Expr::{Closure, ForLoop, Loop, While};
        let repeated = matches!(node, ForLoop(_) | While(_) | Loop(_) | Closure(_));
        self.dynamic += usize::from(repeated);
        syn::visit::visit_expr(self, node);
        self.dynamic -= usize::from(repeated);
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let printed = ["println", "eprintln", "print", "eprint"]
            .iter()
            .any(|name| node.path.is_ident(name));
        if printed {
            let first = node.tokens.clone().into_iter().next();
            if let Some(Ok(lit)) = first.map(|token| syn::parse2::<syn::LitStr>(token.into())) {
                self.sites
                    .push((lit.value(), self.dynamic > 0 || !self.test_function));
            }
        }
        syn::visit::visit_macro(self, node);
    }
}
fn marker_pins(root: &Path, pin: &Pin, artifact: &str, report: &mut Report) {
    let result = (|| -> Result<Emissions, String> {
        let mut identities = BTreeSet::new();
        for selected in &pin.selections {
            if !selected.binary_ids.is_empty() {
                identities.extend(selected.binary_ids.iter().cloned());
                continue;
            }
            let packages = option_values(&selected.args, "-p");
            let (1, Some(package)) = (packages.len(), packages.first()) else {
                return Err(
                    "marker selection requires one package or exact binary_id bindings".into(),
                );
            };
            for binary in option_values(&selected.args, "--test") {
                identities.insert(format!("{package}::{binary}"));
            }
        }
        if identities.is_empty() {
            return Err("marker selection lacks named test binaries".into());
        }
        let mut emissions = Emissions::default();
        for identity in identities {
            let (package, binary) = identity.split_once("::").unwrap();
            let source = match package {
                "xtask" => root.join(format!("xtask/tests/{binary}.rs")),
                _ => root.join(format!("crates/{package}/tests/{binary}.rs")),
            };
            let text =
                fs::read_to_string(&source).map_err(|e| format!("{}: {e}", source.display()))?;
            let file = syn::parse_file(&text).map_err(|e| e.to_string())?;
            emissions.visit_file(&file);
        }
        Ok(emissions)
    })();
    for (marker, expected) in &pin.markers {
        match &result {
            Ok(emissions) => {
                let (actual, dynamic) = emissions
                    .sites
                    .iter()
                    .filter(|site| site.0.contains(marker))
                    .fold((0, false), |(count, dynamic), site| {
                        (count + 1, dynamic || site.1)
                    });
                if actual == 0 {
                    report.add("ci-pins:marker", artifact, "red", format!("{marker:?}: expected {expected} runtime emissions, actual 0 emission sites"));
                } else if dynamic {
                    report.add("ci-pins:marker", artifact, "skip", format!("{marker:?}: {actual} static emission sites; pin {expected} has dynamic loop/closure/helper invocation cardinality, indeterminate locally; CI runtime marker guard remains authoritative"));
                } else {
                    report.add("ci-pins:marker", artifact, if actual == *expected { "pass" } else { "red" }, format!("{marker:?}: expected {expected}, actual {actual} literal emission sites (heuristic, not runtime proof)"));
                }
            }
            Err(e) => report.add(
                "ci-pins:marker",
                artifact,
                "red",
                format!("{marker:?}: {e}"),
            ),
        }
    }
}
