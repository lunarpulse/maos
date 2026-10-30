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
    let command =
        Regex::new(r"\bcargo run\b[^\n]*?(?:-p|--package)\s+xtask\b[^\n]*?\s--\s+([a-z0-9-]+)")
            .map_err(|e| e.to_string())?;
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
            for line in step["run"]
                .as_str()
                .into_iter()
                .flat_map(str::lines)
                .map(str::trim)
            {
                // Summary-table echo strings are not executable gate invocations.
                if !(line.starts_with("cargo run ") || line.starts_with("env ")) {
                    continue;
                }
                if let Some(capture) = command.captures(line) {
                    let name = capture.get(1).ok_or("command capture missing")?.as_str();
                    if name == "invariant-lock" {
                        continue;
                    }
                    if !mirrored.contains(name) {
                        missing.insert(name.to_owned());
                    }
                }
            }
        }
    }
    Ok(missing)
}

#[test]
fn every_standalone_local_blocking_aggregate_gate_is_in_literal_gates() -> Result<(), String> {
    let mirrored = GATES
        .iter()
        .filter_map(|(invocation, _)| invocation.split_ascii_whitespace().next())
        .collect();
    let workflow = include_str!("../../.github/workflows/discipline.yml");
    let registry = include_str!("../gate-registry.toml");
    assert!(missing_local_blocking(workflow, registry, &mirrored)?.is_empty());
    assert!(!mirrored.contains("clippy-ratchet"));
    Ok(())
}

#[test]
fn removing_epic_close_or_adding_a_new_blocking_local_job_reds_the_pin() -> Result<(), String> {
    let mut mirrored: BTreeSet<_> = GATES
        .iter()
        .filter_map(|(i, _)| i.split_ascii_whitespace().next())
        .collect();
    mirrored.remove("check-epic-close-coherence");
    let missing = missing_local_blocking(
        include_str!("../../.github/workflows/discipline.yml"),
        include_str!("../gate-registry.toml"),
        &mirrored,
    )?;
    assert!(missing.contains("check-epic-close-coherence"));
    let plant = "jobs:\n  aggregate:\n    needs: [new-gate, advisory]\n  new-gate:\n    steps:\n    - run: cargo run -q --locked -p xtask -- check-new-local --json\n  advisory:\n    continue-on-error: true\n    steps:\n    - run: cargo run -p xtask -- advisory-only\n";
    assert_eq!(
        missing_local_blocking(plant, "gates = []\n", &mirrored)?,
        BTreeSet::from(["check-new-local".to_owned()])
    );
    Ok(())
}
