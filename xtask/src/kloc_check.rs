use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    pub per_crate: BTreeMap<String, u64>,
    pub over_budget: Vec<String>,
}

pub fn run(config: &str, json: bool) -> Result<(), String> {
    let report = kloc_check(config)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.alarm {
            eprintln!(
                "::warning::NFR-Maint-1 alarm — 16 KLOC threshold reached: current={}",
                report.aggregate
            );
        }
        if !report.passed {
            let mut table = String::from("| Crate | LOC | Budget | Status |\n|---|---|---|---|\n");
            for (crate_name, loc) in &report.per_crate {
                let budget = get_budget(config, crate_name).unwrap_or(0);
                let status = if *loc > budget {
                    "❌ OVER"
                } else {
                    "✅ ok"
                };
                table.push_str(&format!("| {crate_name} | {loc} | {budget} | {status} |\n"));
            }
            eprintln!("NFR-Maint-1 violation: 20 KLOC ceiling breached: current={}, per-crate breakdown:\n{table}", report.aggregate);
        } else {
            println!("kloc-check: PASSED (aggregate={} LOC)", report.aggregate);
        }
    }

    if !report.passed {
        return Err("kloc-check failed".into());
    }

    Ok(())
}

fn kloc_check(config_path: &str) -> Result<Report, String> {
    // Read budget configuration.
    let config_src = fs::read_to_string(config_path)
        .map_err(|e| format!("cannot read {config_path}: {e}"))?;
    let config: toml::Table = config_src
        .parse()
        .map_err(|e| format!("cannot parse {config_path}: {e}"))?;

    let aggregate_alarm = config
        .get("_aggregate_alarm")
        .and_then(|v| v.as_integer())
        .unwrap_or(16000) as u64;
    let aggregate_hardfail = config
        .get("_aggregate_hardfail")
        .and_then(|v| v.as_integer())
        .unwrap_or(20000) as u64;

    let mut budgets = BTreeMap::new();
    for (key, value) in &config {
        if key.starts_with('_') {
            continue;
        }
        if let Some(v) = value.as_integer() {
            budgets.insert(key.clone(), v as u64);
        }
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

    // Run tokei from workspace root. Use PATH lookup (CI installs to /usr/local/bin,
    // devs may have it via cargo install to ~/.cargo/bin).
    let tokei_path = "tokei";

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
            ".",
        ])
        .current_dir(workspace_root)
        .output()
        .map_err(|e| format!("failed to run tokei: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tokei exited with error: {stderr}"));
    }

    let tokei: TokeiOutput =
        serde_json::from_slice(&output.stdout).map_err(|e| format!("tokei JSON parse error: {e}"))?;

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

fn get_budget(config_path: &str, crate_name: &str) -> Option<u64> {
    let config_src = fs::read_to_string(config_path).ok()?;
    let config: toml::Table = config_src.parse().ok()?;
    config.get(crate_name)?.as_integer().map(|v| v as u64)
}

#[cfg(test)]
mod tests {
    include!("tests/kloc_check_tests.rs");
}
