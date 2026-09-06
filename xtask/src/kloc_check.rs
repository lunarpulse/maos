use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const TOKEI_VERSION: &str = "14.0.0";

#[derive(Debug, Deserialize)]
struct TokeiOutput {
    #[serde(rename = "Rust")]
    rust: Option<TokeiLang>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokeiLang {
    code: u64,
    reports: Vec<TokeiReport>,
}

#[derive(Debug, Deserialize)]
struct TokeiReport {
    name: String,
    stats: TokeiStats,
}

#[derive(Debug, Deserialize)]
struct TokeiStats {
    code: u64,
}

#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub passed: bool,
    pub alarm: bool,
    pub aggregate: u64,
    #[serde(skip)]
    aggregate_alarm: u64,
    #[serde(skip)]
    aggregate_hardfail: u64,
    pub per_crate: BTreeMap<String, u64>,
    /// The validated budget map the report was judged against. Carried so the
    /// operator table renders from the SAME parsed-and-validated values the
    /// verdict used, instead of re-reading and re-parsing the config per crate
    /// through a lenient second path (Story 15-2 review R-P13).
    #[serde(skip)]
    budgets: BTreeMap<String, u64>,
    pub over_budget: Vec<String>,
}

pub fn run(config: &str, json: bool) -> Result<(), String> {
    // Hard Guardrail #2 (Story 9.5 D2): docs-site is a non-Cargo Docusaurus
    // project and must contribute ZERO Rust source. The spec mandates an
    // explicit, *enforced* assertion — not reliance on incidence. Fail fast.
    assert_docs_site_zero_rust()?;

    let report = kloc_check(config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        let out = render_operator_output(&report);
        if let Some(line) = out.alarm {
            eprintln!("{line}");
        }
        if let Some(line) = out.failure {
            eprintln!("{line}");
        }
        if let Some(line) = out.passed {
            println!("{line}");
        }
    }

    if !report.passed {
        return Err("kloc-check failed".into());
    }

    Ok(())
}

fn format_aggregate_notice(kind: &str, configured_threshold: u64, current: u64) -> String {
    format!(
        "workspace aggregate KLOC {kind}: configured threshold={configured_threshold}, current={current}"
    )
}

/// Every operator-facing line the non-JSON path emits. `run()` does nothing but
/// dispatch these to stderr/stdout, so a test on this function covers the CALL
/// SITES and not merely the formatter — the gap that let AC6(a) ship without a
/// proven-red control (Story 15-2 review R-P2).
#[derive(Debug, Default)]
struct OperatorOutput {
    alarm: Option<String>,
    failure: Option<String>,
    passed: Option<String>,
}

fn render_operator_output(report: &Report) -> OperatorOutput {
    let mut out = OperatorOutput::default();
    if report.alarm {
        out.alarm = Some(format_aggregate_notice(
            "alarm",
            report.aggregate_alarm,
            report.aggregate,
        ));
    }
    if report.passed {
        out.passed = Some(format!(
            "kloc-check: PASSED (aggregate={} LOC)",
            report.aggregate
        ));
        return out;
    }

    // A per-crate breach is NOT an aggregate breach. Labelling one as the other
    // names a threshold that was never crossed, which is the same lie about the
    // cause that AC6(a) exists to close one level down in the ❌ column
    // (Story 15-2 §2d and review R-P1).
    let headline = if report.aggregate >= report.aggregate_hardfail {
        format_aggregate_notice("hard fail", report.aggregate_hardfail, report.aggregate)
    } else {
        format!(
            "per-crate KLOC ceiling breach: {}; workspace aggregate={} is UNDER its configured threshold={}",
            report.over_budget.join(", "),
            report.aggregate,
            report.aggregate_hardfail
        )
    };

    let mut table = String::from("| Crate | LOC | Budget | Status |\n|---|---|---|---|\n");
    for (crate_name, loc) in &report.per_crate {
        let budget = report.budgets.get(crate_name).copied().unwrap_or(0);
        let status = if *loc > budget { "❌ OVER" } else { "✅ ok" };
        table.push_str(&format!("| {crate_name} | {loc} | {budget} | {status} |\n"));
    }
    out.failure = Some(format!("{headline}; per-crate breakdown:\n{table}"));
    out
}

/// Hard Guardrail #2 (Story 9.5 D2): docs-site is a non-Cargo Docusaurus project
/// and must contribute ZERO Rust source. tokei's `--types Rust` excludes it
/// structurally, but the spec mandates an *explicit, enforced* path-exclusion
/// ("don't rely on incidence"). This walks docs-site/ and fails if any `.rs`
/// file is present (skipping vendored/generated trees).
fn assert_docs_site_zero_rust() -> Result<(), String> {
    let docs_site = Path::new("docs-site");
    if !docs_site.is_dir() {
        return Ok(()); // docs-site absent — nothing to assert
    }
    let mut offenders = Vec::new();
    walk_rust_files(docs_site, &mut offenders);
    if offenders.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "docs-site isolation violation (Story 9.5 Hard Guardrail #2): found {} Rust file(s) \
             under docs-site/ (must be 0): {}",
            offenders.len(),
            offenders.join(", ")
        ))
    }
}

fn walk_rust_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            // Skip vendored / generated trees.
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(
                name,
                "node_modules" | "build" | ".docusaurus" | ".git" | "target"
            ) {
                continue;
            }
            walk_rust_files(&p, out);
        } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(p.display().to_string());
        }
    }
}

fn validate_tokei_version(output: &str) -> Result<(), String> {
    let found = output
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("unrecognized tokei version output: {output:?}"))?;
    if found != TOKEI_VERSION {
        return Err(format!(
            "tokei version mismatch: expected {TOKEI_VERSION}, found {found}"
        ));
    }
    Ok(())
}

fn required_nonnegative_integer(config: &toml::Table, key: &str) -> Result<u64, String> {
    let value = config
        .get(key)
        .ok_or_else(|| format!("missing required integer KLOC threshold `{key}`"))?;
    let integer = value
        .as_integer()
        .ok_or_else(|| format!("KLOC threshold `{key}` must be an integer"))?;
    u64::try_from(integer)
        .map_err(|_| format!("KLOC threshold `{key}` must be non-negative, found {integer}"))
}

/// Production entry point: resolve `tokei` from `PATH` (CI installs to
/// `/usr/local/bin`, devs via `cargo install` to `~/.cargo/bin`).
fn kloc_check(config_path: &str) -> Result<Report, String> {
    kloc_check_with_tokei(config_path, "tokei")
}

/// Seam: `tokei_path` is injectable so the version pin can be exercised at its
/// CALL SITE with a binary that reports a different version, instead of only
/// unit-testing the parser (Story 15-2 review R-P3).
fn kloc_check_with_tokei(config_path: &str, tokei_path: &str) -> Result<Report, String> {
    // Read budget configuration.
    let config_src =
        fs::read_to_string(config_path).map_err(|e| format!("cannot read {config_path}: {e}"))?;
    let config: toml::Table = config_src
        .parse()
        .map_err(|e| format!("cannot parse {config_path}: {e}"))?;

    let aggregate_alarm = required_nonnegative_integer(&config, "_aggregate_alarm")?;
    let aggregate_hardfail = required_nonnegative_integer(&config, "_aggregate_hardfail")?;
    // An alarm above the hard fail can never be observed: the gate errors out
    // before the warning it was supposed to precede. That is the dead-signal
    // state F8 exists to end, reachable by one transposed digit
    // (Story 15-2 review R-P6).
    if aggregate_alarm > aggregate_hardfail {
        return Err(format!(
            "KLOC `_aggregate_alarm` ({aggregate_alarm}) must not exceed `_aggregate_hardfail` ({aggregate_hardfail}): an alarm above the hard fail can never fire"
        ));
    }

    let mut budgets = BTreeMap::new();
    for (key, value) in &config {
        if key.starts_with('_') {
            continue;
        }
        if let Some(table) = value.as_table() {
            // Iteration walks the document root only, so a budget row written
            // BELOW a table header becomes that table's sub-key and is silently
            // inert — no budget, no warning, no error (Story 15-2 §2a, the
            // ship-blocker). `[in_progress_decomposition]` carries only
            // `phase_N = { … }` metadata, so any scalar here is a misplaced
            // budget row and is named rather than swallowed (review R-P7).
            for (nested_key, nested_value) in table {
                if nested_value.as_integer().is_some() || nested_value.as_str().is_some() {
                    return Err(format!(
                        "KLOC budget-shaped key `{nested_key}` is nested inside table `[{key}]`; budget rows must live at the document root"
                    ));
                }
            }
            continue;
        }
        let integer = value
            .as_integer()
            .ok_or_else(|| format!("KLOC budget `{key}` must be an integer"))?;
        let budget = u64::try_from(integer)
            .map_err(|_| format!("KLOC budget `{key}` must be non-negative, found {integer}"))?;
        budgets.insert(key.clone(), budget);
    }

    // Determine workspace root from config path.
    let workspace_root = {
        let p = Path::new(config_path);
        let grandparent = p.parent().and_then(|p| p.parent());
        match grandparent {
            Some(gp) if !gp.as_os_str().is_empty() => gp,
            _ => Path::new("."),
        }
    };

    // Both tokei invocations run from the workspace root so the version the pin
    // approves is resolved by the same lookup that measures the tree
    // (Story 15-2 review R-P8).
    let version_output = Command::new(tokei_path)
        .current_dir(workspace_root)
        .arg("--version")
        .output()
        .map_err(|e| format!("failed to read tokei version: {e}"))?;
    if !version_output.status.success() {
        let stderr = String::from_utf8_lossy(&version_output.stderr);
        return Err(format!("tokei --version exited with error: {stderr}"));
    }
    let version = String::from_utf8_lossy(&version_output.stdout);
    validate_tokei_version(&version)?;

    let output = Command::new(tokei_path)
        .args([
            "--output",
            "json",
            "--types",
            "Rust",
            "-e",
            "target",
            "-e",
            "tests",
            "-e",
            "benches",
            "-e",
            "examples",
            "-e",
            "fuzz",
            "-e",
            "spirits",
            // cfg(test/debug_assertions) fault-injection scaffolding for the private spill
            // transaction; excluded per the production Rust code only intent (ratified
            // 2026-08-03), not a ceiling increase.
            "-e",
            "crates/maos-kernel-core/src/memory/spill_test_faults.rs",
            ".",
        ])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| format!("failed to run tokei: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tokei exited with error: {stderr}"));
    }

    let tokei: TokeiOutput = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("tokei JSON parse error: {e}"))?;

    let rust = tokei.rust.ok_or("tokei returned no Rust statistics")?;

    // Group per-crate.
    let mut per_crate: BTreeMap<String, u64> = BTreeMap::new();
    for report in &rust.reports {
        let crate_name = infer_crate_name(&report.name);
        if crate_name.is_empty() {
            continue;
        }
        *per_crate.entry(crate_name).or_insert(0) += report.stats.code;
    }

    for crate_name in per_crate.keys() {
        if !budgets.contains_key(crate_name) {
            return Err(format!(
                "missing KLOC budget for measured crate `{crate_name}`"
            ));
        }
    }

    // Also include crates with 0 LOC that have budgets.
    for crate_name in budgets.keys() {
        per_crate.entry(crate_name.clone()).or_insert(0);
    }

    let aggregate: u64 = per_crate.values().sum();
    let alarm = aggregate >= aggregate_alarm;
    let mut over_budget = Vec::new();

    if aggregate >= aggregate_hardfail {
        over_budget.push(format!("aggregate {} >= {}", aggregate, aggregate_hardfail));
    }

    for (crate_name, loc) in &per_crate {
        if let Some(&budget) = budgets.get(crate_name) {
            if *loc > budget {
                over_budget.push(format!("{crate_name} {loc} > {budget}"));
            }
        }
    }

    let passed = over_budget.is_empty();

    Ok(Report {
        passed,
        alarm,
        aggregate,
        aggregate_alarm,
        aggregate_hardfail,
        per_crate,
        budgets,
        over_budget,
    })
}

fn infer_crate_name(path: &str) -> String {
    // Path formats from tokei:
    // ./crates/maos-kernel-core/src/lib.rs
    // ./xtask/src/main.rs
    // Also handles non-./ prefixed and absolute paths.
    let stripped = path.strip_prefix("./").unwrap_or(path);

    // Detect any path under crates/<name>/...
    if let Some(rest) = stripped.strip_prefix("crates/") {
        if let Some(idx) = rest.find('/') {
            return rest[..idx].to_string();
        }
        // Edge case: path is exactly "crates/<name>" (e.g., Cargo.toml at crate root).
        return rest.to_string();
    }

    if stripped.starts_with("xtask") {
        return "xtask".to_string();
    }

    // If we can't infer a crate name, return the path as a fallback identifier
    // so the crate shows up in the breakdown rather than being silently dropped.
    if !stripped.is_empty() && !stripped.starts_with("target") && !stripped.starts_with("spirits") {
        // Use the top-level directory as a fallback label.
        let fallback = stripped.split('/').next().unwrap_or(stripped);
        if !fallback.is_empty() {
            return format!("(unknown:{fallback})");
        }
    }

    String::new()
}

#[cfg(test)]
mod tests {
    include!("tests/kloc_check_tests.rs");
}
