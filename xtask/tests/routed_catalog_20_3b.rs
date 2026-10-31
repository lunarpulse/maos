//! AC5 FR63: the real catalog gate proves AST/registry/artifact bijection;
//! this witness independently checks the published kernel rows' semantics.
use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn validate_kernel_rows(catalog: &Value) -> Result<usize, String> {
    let mut count = 0;
    for (code, row) in catalog.as_object().ok_or("catalog must be an object")? {
        if !matches!(row["kernel_or_spirit"].as_str(), Some("kernel" | "both")) {
            continue;
        }
        count += 1;
        let retry = match row["recovery_class"].as_str() {
            Some("retry") => true,
            Some("retry_with_correction" | "reject" | "fix_config" | "escalate") => false,
            _ => return Err(format!("{code}: invalid recovery class")),
        };
        if row["retryable"].as_bool() != Some(retry) {
            return Err(format!("{code}: retryability missing or inconsistent"));
        }
        let cause = match row["severity"].as_str() {
            Some("security") => "security policy enforcement",
            Some("policy") => "admission or runtime policy",
            Some("user") => "invalid user input or request",
            Some("infra") => "infrastructure or environment",
            Some("internal") => "kernel internal invariant",
            _ => return Err(format!("{code}: invalid severity")),
        };
        if row["cause_chain_semantics"].as_str() != Some(cause) {
            return Err(format!(
                "{code}: cause-chain semantics missing or inconsistent"
            ));
        }
        for field in ["rust_path", "cause", "remediation", "version_stability"] {
            if row[field].as_str().is_none_or(|s| s.trim().is_empty()) {
                return Err(format!("{code}: missing {field}"));
            }
        }
    }
    if count == 0 {
        return Err("no kernel error rows".into());
    }
    Ok(count)
}

#[test]
fn every_kernel_catalog_variant_has_retryability_and_cause_chain() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace");
    let out = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .current_dir(root)
        .env_remove("CARGO_MANIFEST_DIR")
        .args(["error-catalog-check", "--json"])
        .output()
        .expect("run real catalog gate");
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value = serde_json::from_slice(&out.stdout).expect("gate JSON");
    assert_eq!(report["passed"], true);
    assert_eq!(report["discovered_count"], report["registry_count"]);
    let catalog: Value = serde_json::from_str(include_str!("../../docs/errors/error-catalog.json"))
        .expect("published catalog");
    assert!(validate_kernel_rows(&catalog).expect("FR63 metadata") > 0);
}

#[test]
fn catalog_semantics_witness_refuses_missing_or_inconsistent_fields() {
    let catalog: Value =
        serde_json::from_str(include_str!("../../docs/errors/error-catalog.json")).unwrap();
    let code = catalog
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, row)| row["kernel_or_spirit"] == "kernel")
        .unwrap()
        .0;
    for field in ["retryable", "cause_chain_semantics", "cause", "remediation"] {
        let mut planted = catalog.clone();
        planted[code].as_object_mut().unwrap().remove(field);
        assert!(validate_kernel_rows(&planted).is_err(), "{field}");
    }
    let mut planted = catalog.clone();
    planted[code]["retryable"] = Value::Bool(!catalog[code]["retryable"].as_bool().unwrap());
    assert!(validate_kernel_rows(&planted).is_err());
    planted = catalog.clone();
    planted[code]["cause_chain_semantics"] = "unrelated cause".into();
    assert!(validate_kernel_rows(&planted).is_err());
}
