//! Story 15-4 AC3 — publication requires a successful discipline aggregate.

use std::io::Read;

use serde::{Deserialize, Serialize};

const CHECK_NAME: &str = "aggregate";

#[derive(Debug, Deserialize)]
struct CheckRunsResponse {
    #[serde(default)]
    check_runs: Vec<CheckRun>,
}

#[derive(Debug, Deserialize)]
struct CheckRun {
    name: String,
    status: String,
    conclusion: Option<String>,
    #[serde(default)]
    head_sha: String,
}

#[derive(Debug, Serialize)]
struct Report<'a> {
    passed: bool,
    check_name: &'a str,
    status: &'a str,
    conclusion: &'a str,
    head_sha: &'a str,
}

pub fn run(check_runs_path: Option<&str>, json: bool) -> Result<(), String> {
    let input = match check_runs_path {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|error| format!("failed to read check-runs response {path}: {error}"))?,
        None => {
            let mut input = String::new();
            std::io::stdin()
                .read_to_string(&mut input)
                .map_err(|error| {
                    format!("failed to read check-runs response from stdin: {error}")
                })?;
            input
        }
    };

    let response: CheckRunsResponse = serde_json::from_str(&input)
        .map_err(|error| format!("invalid GitHub check-runs response: {error}"))?;
    let aggregate_runs: Vec<&CheckRun> = response
        .check_runs
        .iter()
        .filter(|run| run.name == CHECK_NAME)
        .collect();

    if aggregate_runs.is_empty() {
        return Err(
            "release precondition failed: no aggregate check-run exists for this tagged SHA; discipline runs only after the commit reaches main"
                .to_string(),
        );
    }

    // `filter=latest` reduces the response to one run per check name, so index 0 IS
    // the run this release is judged on. Never search for *any* success: a payload
    // holding both the pull_request and the push run for one SHA would let a stale
    // success wave through a failed current aggregate.
    let run = aggregate_runs[0];
    if run.status != "completed" {
        return Err(format!(
            "release precondition failed: aggregate check-run status is {}, not completed",
            run.status
        ));
    }
    if run.conclusion.as_deref() != Some("success") {
        return Err(format!(
            "release precondition failed: aggregate check-run conclusion is {}, not success",
            run.conclusion.as_deref().unwrap_or("null")
        ));
    }
    if json {
        println!(
            "{}",
            serde_json::to_string(&Report {
                passed: true,
                check_name: CHECK_NAME,
                status: &run.status,
                conclusion: "success",
                head_sha: &run.head_sha,
            })
            .map_err(|error| format!("failed to serialize release precondition: {error}"))?
        );
    } else {
        eprintln!("check-release-precondition: PASS — aggregate completed successfully");
    }
    Ok(())
}
