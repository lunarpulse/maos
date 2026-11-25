//! `kloc-check`: measure the production Rust lines of every crate with `tokei`, print the
//! size table, and fail on exactly one condition: `maos-kernel-core` above its row in
//! `xtask/kloc.toml` (the ADR-069 envelope; the row's own bound is pinned by
//! `xtask/tests/recovery_lane_ceiling_rule.rs`).
//!
//! Every other crate is measured and printed, never gated: the per-crate ceilings and
//! `_aggregate_alarm` were removed by operator decision 11 (lean-gates WP-23, confirmed
//! 2026-11-24). Growth outside the kernel is visible in the table, it does not block a PR.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const TOKEI_VERSION: &str = "14.0.0";

/// The one gated row of `xtask/kloc.toml`.
const KERNEL_CRATE: &str = "maos-kernel-core";

/// Keys of the retired threshold scheme. A config that still carries one is refused so a
/// number nothing enforces cannot linger in the file.
const RETIRED_KEYS: [&str; 2] = ["_aggregate_alarm", "_aggregate_hardfail"];

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
    pub aggregate: u64,
    /// The `maos-kernel-core` row of `xtask/kloc.toml`: the only ceiling.
    pub kernel_ceiling: u64,
    pub per_crate: BTreeMap<String, u64>,
    pub over_budget: Vec<String>,
}

pub fn run(config: &str, json: bool) -> Result<(), String> {
    // Hard Guardrail #2 (Story 9.5 D2): docs-site is a non-Cargo Docusaurus
    // project and must contribute ZERO Rust source. The spec mandates an
    // explicit, *enforced* assertion, not reliance on incidence. Fail fast.
    assert_docs_site_zero_rust()?;

    let report = kloc_check(config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        let out = render_operator_output(&report);
        println!("{}", out.table);
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

/// Every operator-facing line the non-JSON path emits. `run()` does nothing but
/// dispatch these to stderr/stdout, so a test on this function covers the CALL
/// SITES and not merely the formatter.
#[derive(Debug, Default)]
struct OperatorOutput {
    table: String,
    failure: Option<String>,
    passed: Option<String>,
}

fn render_operator_output(report: &Report) -> OperatorOutput {
    let mut rows: Vec<(&String, &u64)> = report.per_crate.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    let mut table = String::from("| Crate | tokei LOC | Ceiling |\n|---|---:|---:|\n");
    for (crate_name, loc) in rows {
        let ceiling = if crate_name == KERNEL_CRATE {
            let status = if *loc > report.kernel_ceiling {
                " OVER"
            } else {
                ""
            };
            format!("{}{status}", report.kernel_ceiling)
        } else {
            "-".to_string()
        };
        table.push_str(&format!("| {crate_name} | {loc} | {ceiling} |\n"));
    }
    table.push_str(&format!("| (total) | {} | - |", report.aggregate));

    let mut out = OperatorOutput {
        table,
        ..OperatorOutput::default()
    };
    if report.passed {
        out.passed = Some(format!(
            "kloc-check: PASSED ({KERNEL_CRATE} {} <= {}; aggregate={} LOC, measured, not gated)",
            report.per_crate.get(KERNEL_CRATE).copied().unwrap_or(0),
            report.kernel_ceiling,
            report.aggregate
        ));
    } else {
        out.failure = Some(format!(
            "kernel envelope breach: {}; xtask/kloc.toml bounds only {KERNEL_CRATE} (ADR-069); \
             beyond the envelope is an operator decision, not a grant",
            report.over_budget.join(", ")
        ));
    }
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
        return Ok(()); // docs-site absent: nothing to assert
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

/// Production entry point: resolve `tokei` from `PATH` (CI installs to
/// `/usr/local/bin`, devs via `cargo install` to `~/.cargo/bin`).
fn kloc_check(config_path: &str) -> Result<Report, String> {
    kloc_check_with_tokei(config_path, "tokei")
}

/// Read the kernel ceiling from the parsed config, refusing every other budget-shaped key.
fn kernel_ceiling(config: &toml::Table, config_path: &str) -> Result<u64, String> {
    for key in RETIRED_KEYS {
        if config.contains_key(key) {
            return Err(format!(
                "KLOC config key `{key}` was retired (operator decision 11, lean-gates WP-23): \
                 delete it from {config_path}; only the `{KERNEL_CRATE}` row is gated"
            ));
        }
    }

    let mut ceiling = None;
    for (key, value) in config {
        if key.starts_with('_') {
            continue;
        }
        if let Some(table) = value.as_table() {
            // Iteration walks the document root only, so a row written BELOW a
            // table header becomes that table's sub-key and is silently inert.
            // `[in_progress_decomposition]` carries only `phase_N = { ... }`
            // metadata, so any scalar here is a misplaced row and is named
            // rather than swallowed.
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
        if key != KERNEL_CRATE {
            return Err(format!(
                "KLOC row `{key}` is a per-crate ceiling, removed by operator decision 11 \
                 (lean-gates WP-23): delete it from {config_path}; only `{KERNEL_CRATE}` is gated \
                 and every crate is measured in the printed size table"
            ));
        }
        ceiling = Some(budget);
    }
    ceiling.ok_or_else(|| format!("missing required `{KERNEL_CRATE}` row in {config_path}"))
}

/// Seam: `tokei_path` is injectable so the version pin can be exercised at its
/// CALL SITE with a binary that reports a different version, instead of only
/// unit-testing the parser (Story 15-2 review R-P3).
fn kloc_check_with_tokei(config_path: &str, tokei_path: &str) -> Result<Report, String> {
    // Read the configuration.
    let config_src =
        fs::read_to_string(config_path).map_err(|e| format!("cannot read {config_path}: {e}"))?;
    let config: toml::Table = config_src
        .parse()
        .map_err(|e| format!("cannot parse {config_path}: {e}"))?;
    let kernel_ceiling = kernel_ceiling(&config, config_path)?;

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
    // The gated crate always has a row in the table, even when it measures 0.
    per_crate.entry(KERNEL_CRATE.to_string()).or_insert(0);

    let aggregate: u64 = per_crate.values().sum();
    let mut over_budget = Vec::new();
    let kernel_loc = per_crate[KERNEL_CRATE];
    if kernel_loc > kernel_ceiling {
        over_budget.push(format!("{KERNEL_CRATE} {kernel_loc} > {kernel_ceiling}"));
    }

    Ok(Report {
        passed: over_budget.is_empty(),
        aggregate,
        kernel_ceiling,
        per_crate,
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
