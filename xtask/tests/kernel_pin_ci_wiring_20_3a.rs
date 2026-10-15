#![forbid(unsafe_code)]

//! Pin the real workflow job; each deletion is planted into that parsed tree.
use serde_yaml::Value;
use std::path::Path;

fn workflow() -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    serde_yaml::from_str(
        &std::fs::read_to_string(root.join(".github/workflows/discipline.yml")).unwrap(),
    )
    .unwrap()
}

fn check(workflow: &Value) -> Result<(), &'static str> {
    let steps = workflow["jobs"]["check-kernel-baseline"]["steps"]
        .as_sequence()
        .ok_or("missing steps")?;
    if !steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|uses| uses.starts_with("actions/checkout@"))
            && step["with"]["fetch-depth"].as_u64() == Some(0)
    }) {
        return Err("missing checkout fetch-depth: 0");
    }
    if !steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|uses| uses.starts_with("dtolnay/rust-toolchain@"))
            && step["with"]["components"]
                .as_str()
                .is_some_and(|components| components.split([',', ' ']).any(|c| c == "rustfmt"))
    }) {
        return Err("missing toolchain components: rustfmt");
    }
    if !steps.iter().any(|step| {
        step["name"].as_str() == Some("Run the Story 16-0 kernel-pin content-hash suite")
            && step["run"].as_str().is_some_and(|run| {
                run.split_whitespace()
                    .collect::<Vec<_>>()
                    .windows(2)
                    .any(|pair| pair == ["--test", "kernel_pin_content_hash_16_0"])
            })
    }) {
        return Err("missing named kernel-pin suite step");
    }
    Ok(())
}

#[test]
fn kernel_pin_ci_job_has_all_required_prerequisites() {
    assert_eq!(check(&workflow()), Ok(()));
}

#[test]
fn each_required_kernel_pin_wiring_deletion_reds() {
    let real = workflow();
    assert_eq!(check(&real), Ok(()));
    for (field, expected) in [
        ("fetch-depth", "missing checkout fetch-depth: 0"),
        ("components", "missing toolchain components: rustfmt"),
        ("run", "missing named kernel-pin suite step"),
    ] {
        let mut planted = real.clone();
        let steps = planted["jobs"]["check-kernel-baseline"]["steps"]
            .as_sequence_mut()
            .unwrap();
        let step = steps
            .iter_mut()
            .find(|step| {
                if field == "run" {
                    step["name"].as_str()
                        == Some("Run the Story 16-0 kernel-pin content-hash suite")
                } else {
                    !step["with"][field].is_null()
                }
            })
            .unwrap();
        if field == "run" {
            step.as_mapping_mut().unwrap().remove(Value::from(field));
        } else {
            step["with"]
                .as_mapping_mut()
                .unwrap()
                .remove(Value::from(field));
        }
        assert_eq!(
            check(&planted),
            Err(expected),
            "deleting {field} must red the real-tree checker"
        );
        eprintln!("planted deletion {field}: RED ({expected})");
    }
}
