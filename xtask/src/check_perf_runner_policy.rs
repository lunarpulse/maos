#![forbid(unsafe_code)]
//! Repository policy is defense in depth; operator runner-group and environment
//! protections, not PR-editable code, are the runner admission boundary.
use serde_yaml::Value;
use std::path::Path;
const GATE: &str = "check-perf-runner-policy";
const WORKFLOWS: &str = ".github/workflows";
const PERF: &str = "perf.yml";
const HOST: [&str; 2] = ["self-hosted", "maos-perf"];
const HOSTED: [&str; 9] = [
    "ubuntu-latest",
    "ubuntu-24.04",
    "ubuntu-22.04",
    "ubuntu-26.04",
    "ubuntu-24.04-arm",
    "ubuntu-26.04-arm",
    "windows-latest",
    "macos-latest",
    "macos-14",
];
const MAIN: &str = "github.ref == 'refs/heads/main'";
fn triggers(w: &Value) -> Vec<String> {
    match w.get("on") {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Sequence(s)) => s
            .iter()
            .map(|v| v.as_str().unwrap_or("INVALID").to_owned())
            .collect(),
        Some(Value::Mapping(m)) => m
            .keys()
            .map(|v| v.as_str().unwrap_or("INVALID").to_owned())
            .collect(),
        _ => vec!["INVALID".into()],
    }
}
fn expression(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix("${{")
        .and_then(|s| s.strip_suffix("}}"))
        .unwrap_or(s)
        .trim()
}
fn secret(v: &Value) -> bool {
    match v {
        Value::String(s) => s.split("${{").skip(1).any(|e| {
            e.split("}}").next().is_some_and(|e| {
                e.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .any(|t| t.eq_ignore_ascii_case("secrets"))
            })
        }),
        Value::Sequence(s) => s.iter().any(secret),
        Value::Mapping(m) => m.iter().any(|(k, v)| secret(k) || secret(v)),
        _ => false,
    }
}
fn hosted(job: &Value) -> bool {
    let Some(selector) = job.get("runs-on").and_then(Value::as_str) else {
        return false;
    };
    if HOSTED.contains(&selector) {
        return true;
    }
    let axis = match selector {
        "${{ matrix.os }}" => "os",
        "${{ matrix.image }}" => "image",
        _ => return false,
    };
    let Some(matrix) = job
        .get("strategy")
        .and_then(|v| v.get("matrix"))
        .and_then(Value::as_mapping)
        .filter(|m| m.len() == 1)
    else {
        return false;
    };
    let hosted = |v: &Value| v.as_str().is_some_and(|s| HOSTED.contains(&s));
    if let Some(values) = matrix.get(axis).and_then(Value::as_sequence) {
        return !values.is_empty() && values.iter().all(hosted);
    }
    matrix
        .get("include")
        .and_then(Value::as_sequence)
        .is_some_and(|rows| {
            !rows.is_empty() && rows.iter().all(|v| v.get(axis).is_some_and(hosted))
        })
}
pub fn audit(root: &Path) -> Result<Vec<String>, String> {
    let mut findings = Vec::new();
    let mut seen = false;
    let anchors = regex::Regex::new(r"(?:^|[\s:\[,])[*&][A-Za-z_][A-Za-z0-9_-]*")
        .map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(root.join(WORKFLOWS)).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if !matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("yml" | "yaml")
        ) {
            continue;
        }
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
        seen |= name == PERF;
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let w: Value = match serde_yaml::from_str(&text) {
            Ok(w) => w,
            Err(e) => {
                findings.push(format!("{name}: unparsable: {e}"));
                continue;
            }
        };
        // Do not resolve anchors/aliases differently from GitHub. They are not
        // part of this repository's accepted workflow language.
        if text
            .lines()
            .any(|l| !l.trim_start().starts_with('#') && anchors.is_match(l))
        {
            findings.push(format!("{name}: YAML anchors/aliases refused"));
        }
        let mut runner_indent = None;
        for line in text
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        {
            let indent = line.len() - line.trim_start().len();
            if line.trim_start().starts_with("runs-on:") {
                runner_indent = Some(indent);
            } else if runner_indent.is_some_and(|n| indent <= n) {
                runner_indent = None;
            }
            if runner_indent.is_some() && line.contains('\\') {
                findings.push(format!("{name}: escaped runner labels refused"));
            }
        }
        let Some(jobs) = w.get("jobs").and_then(Value::as_mapping) else {
            findings.push(format!("{name}: jobs missing"));
            continue;
        };
        for (id, job) in jobs {
            if hosted(job) && job.get("uses").is_none() {
                continue;
            }
            let id = id.as_str().unwrap_or("?");
            let prefix = format!("{name}: job {id}");
            let exact_host = job
                .get("runs-on")
                .and_then(Value::as_sequence)
                .is_some_and(|s| {
                    s.len() == 2 && s.iter().zip(HOST).all(|(v, s)| v.as_str() == Some(s))
                });
            if name != PERF || !exact_host || job.get("uses").is_some() {
                findings.push(format!("{prefix}: ambiguous or unapproved host selector; only perf.yml may target [self-hosted, maos-perf]"));
            }
            for event in triggers(&w) {
                if !["push", "schedule", "workflow_dispatch"].contains(&event.as_str()) {
                    findings.push(format!("{prefix}: host workflow trigger {event} refused (including pull_request, pull_request_target, issue_comment)"));
                }
                if event == "push"
                    && w.get("on")
                        .and_then(|v| v.get("push"))
                        .and_then(|v| v.get("branches"))
                        .and_then(Value::as_sequence)
                        .is_none_or(|s| s.len() != 1 || s[0].as_str() != Some("main"))
                {
                    findings.push(format!("{prefix}: push must be restricted to main"));
                }
            }
            let condition = job
                .get("if")
                .and_then(Value::as_str)
                .map(expression)
                .unwrap_or("");
            let accepted = [MAIN.to_owned(), format!("{MAIN} && vars.MAOS_PERF_RUNNER_ENABLED == 'true' && github.event.schedule != '7 1 * * 0'"), format!("{MAIN} && vars.MAOS_PERF_RUNNER_ENABLED == 'true' && github.event.schedule == '7 1 * * 0'")];
            if !accepted.iter().any(|s| s == condition) {
                findings.push(format!("{prefix}: exact main-ref guard required; fork guard or substring is not admission"));
            }
            if job.get("environment").and_then(Value::as_str) != Some("perf-host") {
                findings.push(format!("{prefix}: environment perf-host required"));
            }
            let permissions = job
                .get("permissions")
                .or_else(|| w.get("permissions"))
                .and_then(Value::as_mapping);
            if permissions.is_none_or(|m| {
                m.len() != 1 || m.get("contents").and_then(Value::as_str) != Some("read")
            }) {
                findings.push(format!("{prefix}: must hold exactly contents: read"));
            }
            if secret(job) || w.get("env").is_some_and(secret) {
                findings.push(format!("{prefix}: repository secrets refused"));
            }
        }
    }
    if !seen {
        findings.push("perf.yml: absent".into());
    }
    Ok(findings)
}
pub fn run(json: bool) -> Result<(), String> {
    let findings = audit(&std::env::current_dir().map_err(|e| e.to_string())?)?;
    if json {
        println!(
            "{}",
            serde_json::json!({"gate":GATE,"passed":findings.is_empty(),"findings":findings})
        );
    }
    if findings.is_empty() {
        println!(
            "{GATE}: PASS — structural selectors, trusted triggers, exact guards, read-only host"
        );
        Ok(())
    } else {
        Err(format!("{GATE}: FAIL — {}", findings.join("\n")))
    }
}
