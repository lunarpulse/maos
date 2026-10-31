#![allow(dead_code)]
include!("../src/prepush.rs");

// These CI invocations have no unconditional standalone local form. Their
// exclusions are context, not advisory reclassification of a Blocking gate.
const NO_LOCAL_FORM: &[(&str, &str)] = &[
    (
        "invariant-lock",
        "PR changed-files/reviews/journal context; standalone cassette-age leg is mirrored",
    ),
    (
        "check-mock-not-in-release",
        "CI release binary; local release build is conditional",
    ),
    (
        "epic-19-exit",
        "scene fixture setup plus mutation/twin reconciliation, not a standalone xtask gate",
    ),
    (
        "release-dry-run",
        "CI target matrix, release artifacts and signing context",
    ),
    (
        "check-test-budget",
        "fresh junit: existing conditional prepush test_budget leg owns the local mirror",
    ),
];

fn missing_local_blocking(
    workflow: &str,
    registry: &str,
    mirrored: &BTreeSet<&str>,
) -> Result<BTreeSet<String>, String> {
    let yaml: serde_yaml::Value = serde_yaml::from_str(workflow).map_err(|e| e.to_string())?;
    let registry: toml::Value = toml::from_str(registry).map_err(|e| e.to_string())?;
    let bindings: BTreeMap<_, _> = registry
        .get("ship_gate")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|g| Some((g.get("name")?.as_str()?, g.get("binding")?.as_str()?)))
        .collect();
    let jobs = yaml["jobs"].as_mapping().ok_or("jobs missing")?;
    let needs = yaml["jobs"]["aggregate"]["needs"]
        .as_sequence()
        .ok_or("aggregate.needs missing")?;
    let command = Regex::new(
        r"^(?:env(?:\s+-u\s+[A-Za-z_][A-Za-z_0-9]*)*\s+)?(?:[A-Za-z_][A-Za-z_0-9]*=\S+\s+)*cargo\s+run\s+(.+)$",
    ).map_err(|e| e.to_string())?;
    let direct_xtask = Regex::new(r"(?:^|\s)(?:\S*/)?xtask(?:\s|$)").map_err(|e| e.to_string())?;
    let mut missing = BTreeSet::new();
    for need in needs {
        let name = need.as_str().ok_or("non-string aggregate need")?;
        let job = jobs
            .get(need)
            .ok_or_else(|| format!("missing job {name}"))?;
        if job["continue-on-error"].as_bool() == Some(true)
            || bindings.get(name).is_some_and(|b| *b != "blocking")
            || NO_LOCAL_FORM.iter().any(|(excluded, reason)| {
                *excluded == name && *excluded != "invariant-lock" && !reason.is_empty()
            })
        {
            continue;
        }
        for step in job["steps"].as_sequence().into_iter().flatten() {
            if step["continue-on-error"].as_bool() == Some(true) {
                continue;
            }
            let run = step["run"].as_str().unwrap_or("").replace("\\\n", " ");
            for line in run
                .lines()
                .flat_map(|line| line.split("&&"))
                .flat_map(|line| line.split(';'))
                .map(str::trim)
            {
                if line.starts_with('#') || line.starts_with("echo ") || line.starts_with("printf ")
                {
                    continue;
                }
                // Exact auxiliary selector discovery, not a gate execution. A
                // different command in this job is still scanned/fails closed.
                if name == "expensive-tests" && line == "filter=\"$(cargo run -q -p xtask -- check-test-cost --expensive-filter)\"" {
                    continue;
                }
                // Source paths and cargo test/build commands are not self-executed gates.
                if line.starts_with("cargo test ") || line.starts_with("cargo build ") {
                    continue;
                }
                let cargo_xtask =
                    line.contains("cargo") && line.contains("run") && line.contains("xtask");
                if !cargo_xtask && !direct_xtask.is_match(line) {
                    continue;
                }
                let capture = command
                    .captures(line)
                    .ok_or_else(|| format!("unrecognized xtask invocation in {name}: {line}"))?;
                let words: Vec<_> = capture[1].split_ascii_whitespace().collect();
                let split = words
                    .iter()
                    .position(|a| *a == "--")
                    .ok_or("xtask lacks --")?;
                if !words[..split]
                    .windows(2)
                    .any(|p| p == ["-p", "xtask"] || p == ["--package", "xtask"])
                {
                    return Err(format!("unrecognized xtask package in {name}: {line}"));
                }
                let args: Vec<_> = words[split + 1..]
                    .iter()
                    .copied()
                    .take_while(|a| !matches!(*a, "|" | ">" | "2>" | "2>&1" | ";"))
                    .collect();
                let gate = *args.first().ok_or("xtask command missing")?;
                if gate == "invariant-lock" {
                    continue;
                }
                // Full local fixture production subsumes CI's selected setup artifacts.
                // This exception is only the producer, never a Blocking gate's flags.
                let invocation = if gate == "build-test-fixtures" {
                    let mut i = 1;
                    while i < args.len() {
                        match args[i] {
                            "--release" => i += 1,
                            "--only" if args.get(i + 1).is_some() => i += 2,
                            _ => return Err(format!("unknown fixture setup args: {line}")),
                        }
                    }
                    gate.to_owned()
                } else {
                    normalize_invocation(&args.join(" "))
                };
                if name == "coverage-matrix-nfr-test-3"
                    && invocation == "coverage-matrix --measure-nfr-test-3 --dry-run"
                {
                    continue;
                }
                if !mirrored.contains(invocation.as_str()) {
                    missing.insert(invocation);
                }
            }
        }
    }
    Ok(missing)
}

fn normalize_invocation(invocation: &str) -> String {
    invocation
        .split_ascii_whitespace()
        .filter(|a| *a != "--json")
        .map(|a| a.replace('=', " "))
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn every_standalone_local_blocking_aggregate_gate_is_in_literal_gates() -> Result<(), String> {
    let mirrored: BTreeSet<String> = GATES
        .iter()
        .map(|(invocation, _)| normalize_invocation(invocation))
        .collect();
    let workflow = include_str!("../../.github/workflows/discipline.yml");
    let registry = include_str!("../gate-registry.toml");
    let borrowed = mirrored.iter().map(String::as_str).collect();
    assert!(missing_local_blocking(workflow, registry, &borrowed)?.is_empty());
    assert!(!mirrored.contains("clippy-ratchet"));
    Ok(())
}

#[test]
fn removing_epic_close_or_adding_a_new_blocking_local_job_reds_the_pin() -> Result<(), String> {
    let mut mirrored: BTreeSet<_> = GATES.iter().map(|(i, _)| normalize_invocation(i)).collect();
    mirrored.remove("check-epic-close-coherence");
    let missing = missing_local_blocking(
        include_str!("../../.github/workflows/discipline.yml"),
        include_str!("../gate-registry.toml"),
        &mirrored.iter().map(String::as_str).collect(),
    )?;
    assert!(missing.contains("check-epic-close-coherence"));
    let plant = "jobs:\n  aggregate:\n    needs: [new-gate, advisory]\n  new-gate:\n    steps:\n    - run: cargo run -q --locked -p xtask -- check-new-local --json\n  advisory:\n    continue-on-error: true\n    steps:\n    - run: cargo run -p xtask -- advisory-only\n";
    assert_eq!(
        missing_local_blocking(
            plant,
            "gates = []\n",
            &mirrored.iter().map(String::as_str).collect()
        )?,
        BTreeSet::from(["check-new-local".to_owned()])
    );
    Ok(())
}

fn plant_job(run: &str) -> String {
    format!(
        "jobs:\n  aggregate:\n    needs: [new-gate]\n  new-gate:\n    steps:\n    - run: |\n{}",
        run.lines()
            .map(|l| format!("        {l}\n"))
            .collect::<String>()
    )
}

#[test]
fn assignments_env_continuations_and_multiline_xtasks_cannot_evade_pin() -> Result<(), String> {
    for run in [
        "VAR=1 cargo run -p xtask -- check-new-local --json",
        "env VAR=1 cargo run -p xtask -- check-new-local",
        "env -u CI VAR=1 cargo run -p xtask -- check-new-local",
        "cargo run \\\n  -p xtask \\\n  -- check-new-local",
        "set -e\ncargo run -p xtask -- check-new-local",
        "echo harmless && VAR=1 cargo run -p xtask -- check-new-local",
        "cargo test -p xtask; VAR=1 cargo run -p xtask -- check-new-local",
    ] {
        assert_eq!(
            missing_local_blocking(&plant_job(run), "", &BTreeSet::new())?,
            BTreeSet::from(["check-new-local".to_owned()]),
            "{run}"
        );
    }
    for run in [
        "cargo run\n -p xtask -- check-new-local",
        "bash -c 'cargo run -p xtask -- check-new-local'",
        "wrapper cargo run -p xtask -- check-new-local",
        "cargo run --package=xtask -- check-new-local",
        "target/debug/xtask check-new-local",
    ] {
        assert!(
            missing_local_blocking(&plant_job(run), "", &BTreeSet::new()).is_err(),
            "{run}"
        );
    }
    Ok(())
}

#[test]
fn missing_check_flag_or_wrong_baseline_value_reds_argument_pin() -> Result<(), String> {
    for (ci, wrong) in [
        ("templates-regen --check --json", "templates-regen"),
        (
            "abi-diff --base abi-baseline/v1-pre-bump.txt --json",
            "abi-diff --base wrong.txt",
        ),
        (
            "calibrate --corpus calibration-seed-v0.1 --n 100 --p 0.95",
            "calibrate --corpus calibration-seed-v0.1 --n 10 --p 0.95",
        ),
    ] {
        let workflow = plant_job(&format!("cargo run --locked -p xtask -- {ci}"));
        assert_eq!(
            missing_local_blocking(&workflow, "", &BTreeSet::from([wrong]))?,
            BTreeSet::from([normalize_invocation(ci)]),
            "{wrong}"
        );
        let correct = normalize_invocation(ci);
        assert!(
            missing_local_blocking(&workflow, "", &BTreeSet::from([correct.as_str()]))?.is_empty()
        );
    }
    Ok(())
}

#[test]
fn auxiliary_measurement_jobs_cannot_hide_an_additional_gate() -> Result<(), String> {
    for job in ["coverage-matrix-nfr-test-3", "expensive-tests"] {
        let workflow =
            plant_job("VAR=1 cargo run -p xtask -- check-new-local").replace("new-gate", job);
        assert_eq!(
            missing_local_blocking(&workflow, "", &BTreeSet::new())?,
            BTreeSet::from(["check-new-local".to_owned()])
        );
    }
    Ok(())
}
