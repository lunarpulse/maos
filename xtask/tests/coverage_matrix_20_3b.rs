use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

const EVIDENCE: &str = "../../_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/landing-2/";

#[derive(serde::Deserialize)]
struct SpikeRegistry {
    coverage_gate: Vec<SpikeGate>,
}
#[derive(serde::Deserialize)]
struct SpikeGate {
    name: String,
    covers: Vec<SpikeCover>,
}
#[derive(serde::Deserialize)]
struct SpikeCover {
    requirement: String,
    position: usize,
}

#[test]
fn spike_source_plus_per_gate_covers_is_byte_identical_before_changes() -> Result<(), String> {
    let evidence = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(EVIDENCE);
    let source =
        std::fs::read_to_string(evidence.join("spike-source.yaml")).map_err(|e| e.to_string())?;
    let registry: SpikeRegistry = toml::from_str(
        &std::fs::read_to_string(evidence.join("spike-covers.toml")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let mut covers: BTreeMap<String, BTreeMap<usize, String>> = BTreeMap::new();
    for gate in registry.coverage_gate {
        for cover in gate.covers {
            assert!(covers
                .entry(cover.requirement)
                .or_default()
                .insert(cover.position, gate.name.clone())
                .is_none());
        }
    }
    let mut output = String::new();
    let mut requirement = "";
    let mut count = 0;
    for line in source.lines() {
        if line.starts_with("  ") && !line.starts_with("    ") && line.ends_with(':') {
            requirement = line.trim().trim_end_matches(':');
            count += 1;
        }
        if let Some(style) = line.strip_prefix("    # gate-list: ") {
            let names = covers.remove(requirement).unwrap_or_default();
            for (expected, actual) in names.keys().enumerate() {
                assert_eq!(expected, *actual);
            }
            if style == "inline" {
                output.push_str(&format!(
                    "    gates: [{}]\n",
                    names.into_values().collect::<Vec<_>>().join(", ")
                ));
            } else {
                let indent: usize = style.parse::<usize>().map_err(|e| e.to_string())?;
                assert!(matches!(indent, 4 | 6));
                output.push_str("    gates:\n");
                for name in names.into_values() {
                    output.push_str(&format!("{}- {name}\n", " ".repeat(indent)));
                }
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    assert!(covers.is_empty(), "unrendered gate claims");
    assert_eq!(count, 198);
    let original =
        std::fs::read_to_string(evidence.join("spike-original.yaml")).map_err(|e| e.to_string())?;
    assert_eq!(output.as_bytes(), original.as_bytes());
    assert_ne!(
        output.replace("mode: warning", "mode: hard"),
        original,
        "intentional changes must not be mistaken for the historical oracle"
    );
    Ok(())
}

#[test]
fn committed_hand_kept_matrix_is_hard_and_all_references_resolve() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("workspace parent missing")?;
    let matrix: serde_yaml::Value =
        serde_yaml::from_str(include_str!("../../tests/coverage-matrix.yaml"))
            .map_err(|e| e.to_string())?;
    assert_eq!(matrix["mode"].as_str(), Some("hard"));
    for id in ["NFR-Rel-7", "NFR-Scale-2", "NFR-Test-8"] {
        assert_eq!(matrix["coverage"][id]["phase"].as_str(), Some("v2.0+"));
        assert!(matrix["coverage"][id]["notes"]
            .as_str()
            .is_some_and(|n| n.contains("PRD delivery phase remains")));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root)
        .args(["coverage-matrix", "--json"])
        .output()
        .map_err(|e| e.to_string())?;
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    assert_eq!(report["passed"].as_bool(), Some(true));
    assert_eq!(report["violations"].as_array().map(Vec::len), Some(0));
    Ok(())
}

#[test]
fn no_go_keeps_the_live_completeness_control_and_documents_why() {
    let spike = include_str!("../../_bmad-output/implementation-artifacts/20-3b-gate-retirement-and-coverage-generator-evidence/landing-2/spike.md");
    assert!(spike.contains("NO-GO"));
    assert!(spike.contains("**43**"));
    assert!(include_str!("../src/main.rs").contains("CheckCoverageMatrixCompleteness"));
    assert!(include_str!("../src/prepush.rs").contains("\"check-coverage-matrix-completeness\""));
    assert!(include_str!("../../.github/workflows/discipline.yml")
        .contains("  check-coverage-matrix-completeness:"));
}
