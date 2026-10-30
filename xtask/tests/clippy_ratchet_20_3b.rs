use xtask::clippy_ratchet::{compare, count, Counts};

const FIXTURE: &str = include_str!("fixtures/clippy-ratchet.jsonl");

#[test]
fn repeated_targets_count_as_occurrences_not_source_distinct() -> Result<(), String> {
    assert_eq!(
        count(FIXTURE.as_bytes())?,
        Counts {
            occurrences: 3,
            source_distinct: 2
        }
    );
    Ok(())
}

#[test]
fn every_t0_identity_component_affects_source_distinct() -> Result<(), String> {
    let warning = FIXTURE.lines().nth(1).ok_or("fixture warning missing")?;
    for (from, to) in [
        ("\"package_id\":\"pkg\"", "\"package_id\":\"other\""),
        ("clippy::sample", "clippy::different"),
        ("sample warning", "another warning"),
        ("src/lib.rs", "src/other.rs"),
        ("\"line_start\":1", "\"line_start\":2"),
        ("\"column_start\":2", "\"column_start\":3"),
        ("\"line_end\":3", "\"line_end\":4"),
        ("\"column_end\":4", "\"column_end\":5"),
    ] {
        let stream = format!(
            "{warning}\n{}\n{{\"reason\":\"build-finished\",\"success\":true}}\n",
            warning.replace(from, to)
        );
        assert_eq!(
            count(stream.as_bytes())?,
            Counts {
                occurrences: 2,
                source_distinct: 2
            },
            "{from}"
        );
    }
    Ok(())
}

#[test]
fn ratchet_fails_if_either_count_grows_and_accepts_equal_or_lower() {
    let baseline = Counts {
        occurrences: 1411,
        source_distinct: 884,
    };
    assert!(compare(&baseline, &baseline).is_ok());
    assert!(compare(
        &Counts {
            occurrences: 1410,
            source_distinct: 883
        },
        &baseline
    )
    .is_ok());
    assert!(compare(
        &Counts {
            occurrences: 1412,
            source_distinct: 884
        },
        &baseline
    )
    .is_err());
    assert!(compare(
        &Counts {
            occurrences: 1411,
            source_distinct: 885
        },
        &baseline
    )
    .is_err());
}

#[test]
fn malformed_truncated_or_failed_clippy_stream_is_not_zero_debt() {
    for bad in [
        "",
        "not json\n",
        "{\"reason\":\"build-finished\",\"success\":false}\n",
        "{\"reason\":\"compiler-artifact\"}\n",
    ] {
        assert!(count(bad.as_bytes()).is_err());
    }
}

#[test]
fn advisory_lane_is_enrolled_but_not_in_blocking_fanin() -> Result<(), String> {
    let workflow: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../.github/workflows/discipline.yml"))
            .map_err(|e| e.to_string())?;
    let job = &workflow["jobs"]["clippy-advisory"];
    assert_eq!(job["continue-on-error"].as_bool(), Some(true));
    let needs = workflow["jobs"]["aggregate"]["needs"]
        .as_sequence()
        .ok_or("aggregate needs missing")?;
    assert!(!needs.iter().any(|n| n.as_str() == Some("clippy-advisory")));
    let steps = job["steps"].as_sequence().ok_or("clippy steps missing")?;
    let runs = steps
        .iter()
        .filter_map(|s| s["run"].as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(runs.contains("cargo clippy --workspace --all-targets --locked --message-format=json"));
    assert!(runs.contains("clippy-ratchet --input clippy.jsonl"));
    assert!(!runs.contains("-D warnings"));
    let baseline: Counts =
        toml::from_str(include_str!("../clippy-baseline.toml")).map_err(|e| e.to_string())?;
    assert_eq!(
        baseline,
        Counts {
            occurrences: 1411,
            source_distinct: 884
        }
    );
    Ok(())
}
