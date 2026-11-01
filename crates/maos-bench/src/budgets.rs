#![forbid(unsafe_code)]
// Included by `#[path]` into kernel-core benches and a maos-bin test, each of
// which uses a subset (the `tests/harness/fixtures.rs` pattern).
#![allow(dead_code)]

//! Story 20-7 AC3 — the harnesses' and benches' reader of the one budget file,
//! `xtask/perf-budgets.toml` (ADR-068 §5). No budget literal lives in this
//! crate; `xtask`'s `budget_literals` scan proves it. Tier W benches print one
//! `MAOS-PERF {"metric","value"}` line per measured metric (the runner's
//! ledger append reads them) and exit non-zero only on a breach of a row whose
//! disposition at the file's phase is `blocking`.

const FILE: &str = include_str!("../../../xtask/perf-budgets.toml");
const PHASES: [&str; 4] = ["v1_0", "v1_5", "v2_0", "v2_2"];

/// One metric row as the benches need it.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    pub value: f64,
    pub is_max: bool,
    pub binding: bool,
}

fn row(id: &str) -> toml::Table {
    let file: toml::Table = FILE.parse().expect("perf-budgets.toml parses");
    let phase = file["protocol"]["phase"].as_str().expect("protocol.phase");
    let metrics = file["metric"].as_array().expect("[[metric]] rows");
    let metric = metrics
        .iter()
        .filter_map(toml::Value::as_table)
        .find(|metric| metric["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("perf-budgets.toml has no metric `{id}`"));
    let mut row = metric.clone();
    let binding = PHASES.iter().position(|p| *p == phase).and_then(|at| {
        PHASES[..=at]
            .iter()
            .rev()
            .find_map(|p| metric["disposition"].get(*p).and_then(toml::Value::as_str))
    }) == Some("blocking");
    row.insert("binding".into(), toml::Value::Boolean(binding));
    row
}

pub fn budget(id: &str) -> Budget {
    let row = row(id);
    let value = match &row["budget"] {
        toml::Value::Integer(n) => *n as f64,
        toml::Value::Float(f) => *f,
        other => panic!("metric `{id}` budget is not a number: {other}"),
    };
    Budget {
        value,
        is_max: row["direction"].as_str() == Some("max"),
        binding: row["binding"].as_bool() == Some(true),
    }
}

/// The budget as whole microseconds (latency rows in `us`).
pub fn budget_us(id: &str) -> u64 {
    budget(id).value as u64
}

/// Print the metric line and judge `value` against the row. `false` only when
/// the row breaches and is binding at the file's phase; an advisory breach
/// prints `WOULD BLOCK` and still returns `true`.
pub fn report(id: &str, value: f64) -> bool {
    println!(
        "MAOS-PERF {}",
        serde_json::json!({ "metric": id, "value": value })
    );
    let budget = budget(id);
    let breached = if budget.is_max {
        value > budget.value
    } else {
        value < budget.value
    };
    if breached {
        eprintln!(
            "{id}: {value} breaches budget {} ({})",
            budget.value,
            if budget.binding {
                "BLOCKING"
            } else {
                "WOULD BLOCK once binding"
            }
        );
    }
    !(breached && budget.binding)
}

/// Exit non-zero when any reported metric breached a binding budget.
pub fn finish(all_within: bool) -> std::process::ExitCode {
    if all_within {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
