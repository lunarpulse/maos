#![forbid(unsafe_code)]

//! Story 20-7 AC2 — `check-perf-runner-policy` (ADR-068 §4): the stable
//! performance runner only ever runs trusted code. On a public repository a
//! self-hosted runner that executes fork code is remote code execution on that
//! host, so this gate is the only thing between a fork pull request and it.
//!
//! Refusals (each proven red by a plant in `xtask/tests/check_perf_runner_policy_20_7.rs`):
//!
//! 1. a workflow other than `perf.yml` that mentions the `maos-perf` label;
//! 2. `perf.yml` absent, or triggered by anything but `push` to `main`,
//!    `schedule`, `workflow_dispatch` and `pull_request` (so never
//!    `pull_request_target`, `workflow_run`, …);
//! 3. a `maos-perf` job in a `pull_request` workflow without the job-level
//!    same-repository guard;
//! 4. a `maos-perf` job whose permissions are not exactly `contents: read`
//!    (absent permissions are the token's wider default), or that reads `secrets.`.

use std::path::Path;

use serde_yaml::Value;

const GATE: &str = "check-perf-runner-policy";
const LABEL: &str = "maos-perf";
const WORKFLOWS: &str = ".github/workflows";
const PERF: &str = "perf.yml";
const FORK_GUARD: &str = "github.event.pull_request.head.repo.full_name == github.repository";
const TRIGGERS: [&str; 4] = ["push", "schedule", "workflow_dispatch", "pull_request"];

fn key<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value.get(name)
}

/// `on:` parses as the boolean `true` under YAML 1.1 loaders; serde_yaml keeps `on`.
fn triggers(workflow: &Value) -> Vec<String> {
    let on = key(workflow, "on").or_else(|| workflow.get(Value::Bool(true)));
    match on {
        Some(Value::String(name)) => vec![name.clone()],
        Some(Value::Sequence(names)) => names
            .iter()
            .filter_map(|name| name.as_str().map(str::to_owned))
            .collect(),
        Some(Value::Mapping(map)) => map
            .keys()
            .filter_map(|name| name.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

fn targets_runner(job: &Value) -> bool {
    key(job, "runs-on")
        .and_then(|runs_on| serde_yaml::to_string(runs_on).ok())
        .is_some_and(|text| text.contains(LABEL))
}

/// Exactly `{contents: read}`; anything else (or nothing) is wider.
fn contents_read_only(permissions: Option<&Value>) -> bool {
    let Some(Value::Mapping(map)) = permissions else {
        return false;
    };
    map.len() == 1 && map.get("contents").and_then(Value::as_str) == Some("read")
}

fn audit_perf(workflow: &Value, findings: &mut Vec<String>) {
    let events = triggers(workflow);
    for event in &events {
        if !TRIGGERS.contains(&event.as_str()) {
            findings.push(format!(
                "{WORKFLOWS}/{PERF}: trigger `{event}` can reach the runner with code a fork controls; only {TRIGGERS:?} are allowed"
            ));
        }
    }
    let on = key(workflow, "on").or_else(|| workflow.get(Value::Bool(true)));
    if let Some(branches) = on
        .and_then(|on| key(on, "push"))
        .and_then(|push| key(push, "branches"))
    {
        if branches
            .as_sequence()
            .is_none_or(|b| b.iter().any(|n| n.as_str() != Some("main")))
        {
            findings.push(format!(
                "{WORKFLOWS}/{PERF}: `push` must be restricted to `main`"
            ));
        }
    } else if events.iter().any(|event| event == "push") {
        findings.push(format!(
            "{WORKFLOWS}/{PERF}: `push` must name `branches: [main]`"
        ));
    }
    let pull_request = events.iter().any(|event| event == "pull_request");
    let Some(Value::Mapping(jobs)) = key(workflow, "jobs") else {
        return;
    };
    for (name, job) in jobs {
        if !targets_runner(job) {
            continue;
        }
        let name = name.as_str().unwrap_or("?");
        let guard = key(job, "if")
            .and_then(Value::as_str)
            .is_some_and(|condition| condition.contains(FORK_GUARD));
        if pull_request && !guard {
            findings.push(format!(
                "{WORKFLOWS}/{PERF}: job `{name}` runs on `{LABEL}` in a pull_request workflow without the job-level fork guard `{FORK_GUARD}`"
            ));
        }
        let permissions = key(job, "permissions").or_else(|| key(workflow, "permissions"));
        if !contents_read_only(permissions) {
            findings.push(format!(
                "{WORKFLOWS}/{PERF}: job `{name}` on `{LABEL}` must hold exactly `permissions: {{contents: read}}` (no write token on the host)"
            ));
        }
        if serde_yaml::to_string(job).is_ok_and(|text| text.contains("secrets.")) {
            findings.push(format!(
                "{WORKFLOWS}/{PERF}: job `{name}` on `{LABEL}` reads a repository secret"
            ));
        }
    }
}

/// Every finding over the workflows under `root`; empty means the policy holds.
pub fn audit(root: &Path) -> Result<Vec<String>, String> {
    let dir = root.join(WORKFLOWS);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|error| format!("{GATE}: cannot read {WORKFLOWS}: {error}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("yml" | "yaml")
            )
        })
        .collect();
    files.sort();
    let mut findings = Vec::new();
    let mut perf_seen = false;
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_owned();
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("{GATE}: cannot read {name}: {error}"))?;
        if name == PERF {
            perf_seen = true;
            match serde_yaml::from_str::<Value>(&text) {
                Ok(workflow) => audit_perf(&workflow, &mut findings),
                Err(error) => findings.push(format!("{WORKFLOWS}/{name}: unparsable: {error}")),
            }
        } else if text
            .lines()
            .any(|line| !line.trim_start().starts_with('#') && line.contains(LABEL))
        {
            findings.push(format!(
                "{WORKFLOWS}/{name}: names the `{LABEL}` runner; only {PERF} may"
            ));
        }
    }
    if !perf_seen {
        findings.push(format!("{WORKFLOWS}/{PERF}: absent"));
    }
    Ok(findings)
}

pub fn run(json: bool) -> Result<(), String> {
    let root = std::env::current_dir().map_err(|error| format!("{GATE}: no cwd: {error}"))?;
    let findings = audit(&root)?;
    if json {
        println!(
            "{}",
            serde_json::json!({ "gate": GATE, "passed": findings.is_empty(), "findings": findings })
        );
    }
    if findings.is_empty() {
        if !json {
            println!("{GATE}: PASS — only {PERF} targets `{LABEL}`, trusted triggers, fork guard, contents: read");
        }
        return Ok(());
    }
    let message = format!(
        "{GATE}: FAIL — {} finding(s):\n{}",
        findings.len(),
        findings
            .iter()
            .map(|f| format!("- {f}\n"))
            .collect::<String>()
    );
    crate::gate_common::emit_command(json, "error", &message);
    if !json {
        eprintln!("{message}");
    }
    Err(message)
}
