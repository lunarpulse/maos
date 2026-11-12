//! Story 15-4 AC3 — publication requires a successful discipline aggregate.
//! Story 20-5 AC4 — and, when `--fuzz-ledger` is given, a complete T3 fuzz chain for the
//! released SHA (the `fuzz-prerelease.yml` collector's rule, one source for both).
//! Operator ruling 2026-11-12 (rc = b) — and, when `--perf-verdicts` is given, the
//! `check-perf-verdict --json` rows for the released SHA: FAIL / INDETERMINATE are always
//! refused, ABSENT only under `--perf-absent-ok` (release candidates), else all PASS.

use std::collections::BTreeMap;
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
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    t3_fuzz_chain: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    perf_verdicts: bool,
}

#[derive(Debug, Deserialize)]
struct PerfReport {
    sha: String,
    verdicts: Vec<PerfRow>,
}

#[derive(Debug, Deserialize)]
struct PerfRow {
    metric: String,
    verdict: String,
}

/// Refuse `sha` unless the `check-perf-verdict --json` report is for exactly that commit,
/// has rows, and every row is PASS (or ABSENT when `absent_ok`). A tool crash prints no
/// JSON and an unknown verdict is not PASS, so neither can be read as ABSENT.
fn check_perf_verdicts(path: &str, sha: &str, absent_ok: bool) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read perf verdicts {path}: {error}"))?;
    let report: PerfReport = serde_json::from_str(&text)
        .map_err(|error| format!("invalid perf verdicts {path}: {error}"))?;
    if report.sha != sha || report.verdicts.is_empty() {
        return Err(format!(
            "release precondition failed: perf verdicts {path} are for {:?} with {} rows, not rows for {sha}",
            report.sha,
            report.verdicts.len()
        ));
    }
    let refused: Vec<String> = report
        .verdicts
        .iter()
        .filter(|row| row.verdict != "PASS" && !(absent_ok && row.verdict == "ABSENT"))
        .map(|row| format!("{}={}", row.metric, row.verdict))
        .collect();
    if refused.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "release precondition failed: perf verdicts at {sha} not PASS: {}",
            refused.join(", ")
        ))
    }
}

/// The T3 targets `fuzz-prerelease.yml` chains (PRD NFR-Sec-5/6).
const T3_TARGETS: [&str; 3] = ["manifest_parser", "frame_deser", "fuzz_exec_deps_parse_elf"];
/// A chain is links `1..=T3_LINKS` of ONE workflow run (`run_id` = `<run>-<link>`).
const T3_LINKS: u32 = 5;
/// 24 h of libFuzzer-measured seconds per target, never configured time.
const T3_SECONDS: u64 = 86_400;

#[derive(Debug, Deserialize)]
struct FuzzLedger {
    schema_version: u32,
    records: Vec<FuzzRecord>,
}

#[derive(Debug, Deserialize)]
struct FuzzRecord {
    target: String,
    tier: String,
    commit: String,
    cpu_seconds: u64,
    run_id: String,
}

/// Refuse `sha` unless every T3 target has one run holding all five links at exactly that
/// commit with at least 24 h of measured seconds. A link recorded twice (a re-run of the
/// same run id) counts once, at its larger measurement; seconds never sum across runs.
fn check_t3_fuzz_chain(ledger_path: &str, sha: &str) -> Result<(), String> {
    if sha.is_empty() {
        return Err(
            "release precondition failed: the aggregate check-run carries no head_sha, so no T3 fuzz chain can be matched to a commit"
                .to_string(),
        );
    }
    let text = std::fs::read_to_string(ledger_path)
        .map_err(|error| format!("failed to read fuzz ledger {ledger_path}: {error}"))?;
    let ledger: FuzzLedger = serde_json::from_str(&text)
        .map_err(|error| format!("invalid fuzz ledger {ledger_path}: {error}"))?;
    if ledger.schema_version != 1 {
        return Err(format!(
            "fuzz ledger {ledger_path} schema_version {} is not 1",
            ledger.schema_version
        ));
    }
    for target in T3_TARGETS {
        let mut runs: BTreeMap<&str, BTreeMap<u32, u64>> = BTreeMap::new();
        for record in ledger
            .records
            .iter()
            .filter(|r| r.target == target && r.tier == "T3" && r.commit == sha)
        {
            let Some((run, link)) = record.run_id.rsplit_once('-') else {
                continue;
            };
            let Ok(link) = link.parse::<u32>() else {
                continue;
            };
            if (1..=T3_LINKS).contains(&link) {
                let seconds = runs.entry(run).or_default().entry(link).or_default();
                *seconds = (*seconds).max(record.cpu_seconds);
            }
        }
        let complete = runs.values().any(|links| {
            links.len() == T3_LINKS as usize && links.values().sum::<u64>() >= T3_SECONDS
        });
        if !complete {
            return Err(format!(
                "release precondition failed: no complete T3 fuzz chain for {target} at {sha} ({T3_LINKS} links of one run, >= {T3_SECONDS} measured libFuzzer seconds); the operator dispatches fuzz-prerelease at this SHA before tagging"
            ));
        }
    }
    Ok(())
}

pub fn run(
    check_runs_path: Option<&str>,
    fuzz_ledger: Option<&str>,
    perf_verdicts: Option<&str>,
    perf_absent_ok: bool,
    json: bool,
) -> Result<(), String> {
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
    if let Some(ledger) = fuzz_ledger {
        check_t3_fuzz_chain(ledger, &run.head_sha)?;
    }
    if let Some(verdicts) = perf_verdicts {
        check_perf_verdicts(verdicts, &run.head_sha, perf_absent_ok)?;
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
                t3_fuzz_chain: fuzz_ledger.is_some(),
                perf_verdicts: perf_verdicts.is_some(),
            })
            .map_err(|error| format!("failed to serialize release precondition: {error}"))?
        );
    } else {
        eprintln!("check-release-precondition: PASS — aggregate completed successfully");
        if fuzz_ledger.is_some() {
            eprintln!(
                "check-release-precondition: PASS — complete T3 fuzz chain for every target at {}",
                run.head_sha
            );
        }
        if perf_verdicts.is_some() {
            eprintln!(
                "check-release-precondition: PASS — perf verdicts at {} ({})",
                run.head_sha,
                if perf_absent_ok {
                    "ABSENT accepted"
                } else {
                    "all PASS"
                }
            );
        }
    }
    Ok(())
}
