#![forbid(unsafe_code)]

//! Story 2.2 spirit-boundary invariant test harness.
//!
//! Parses `tests/corpora/spirit-boundary-v0.1.jsonl` and asserts each case
//! against the Rust types that implement the FR17/FR58 contracts.
//! Cites: Story 2.2 AC6/AC7, FR17, FR58, Story 0.3 corpus-pinning contract.

use std::fs;
use std::path::PathBuf;

use maos_domain::invariants::i1::Scope;
use maos_kernel_core::security::{
    capabilities_required_to_scopes, CapabilitiesRequired, OutputShape, OutputShapePredicate,
    OutputShapeViolation,
};
use maos_spirit_abi::compliance::ComplianceClaimEnvelope;

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseLine {
    id: String,
    class: CaseClass,
    input: serde_json::Value,
    expected_outcome: serde_json::Value,
    #[serde(default)]
    notes: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum CaseClass {
    CapabilityDeclaration,
    ComplianceEmit,
    OutputShape,
}

fn corpus_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/corpora/spirit-boundary-v0.1.jsonl"
    ))
}

fn manifest_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/corpora/MANIFEST.toml"
    ))
}

#[test]
fn spirit_boundary_invariants() {
    let jsonl_path = corpus_path();
    let manifest_path = manifest_path();

    let jsonl_data =
        fs::read_to_string(&jsonl_path).unwrap_or_else(|e| panic!("cannot read corpus: {e}"));

    // Verify SHA-256 against MANIFEST.toml
    let computed_sha = sha256_hex(&jsonl_data);
    let manifest_src =
        fs::read_to_string(&manifest_path).unwrap_or_else(|e| panic!("cannot read manifest: {e}"));
    let manifest: toml::Value = manifest_src.parse().expect("MANIFEST.toml must parse");
    let expected_sha = manifest
        .get("corpus")
        .and_then(|c| c.get("spirit-boundary-v0.1"))
        .and_then(|c| c.get("sha256"))
        .and_then(|s| s.as_str())
        .expect("MANIFEST.toml must contain [corpus.\"spirit-boundary-v0.1\"] sha256");
    assert_eq!(
        computed_sha, expected_sha,
        "corpus SHA-256 mismatch: expected {expected_sha}, got {computed_sha}"
    );

    let mut errors = Vec::new();
    for (idx, line) in jsonl_data.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let case: CaseLine = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line {}: JSON parse error: {e}", idx + 1));

        let result = match case.class {
            CaseClass::CapabilityDeclaration => handle_capability_declaration(&case),
            CaseClass::ComplianceEmit => handle_compliance_emit(&case),
            CaseClass::OutputShape => handle_output_shape(&case),
        };

        if let Err(msg) = result {
            errors.push(format!("case {} failed: {}", case.id, msg));
        }
    }

    if !errors.is_empty() {
        panic!("\n{}", errors.join("\n"));
    }
}

fn handle_capability_declaration(case: &CaseLine) -> Result<(), String> {
    let manifest_toml = case
        .input
        .get("manifest_toml")
        .and_then(|v| v.as_str())
        .ok_or("missing input.manifest_toml")?;

    let caps = match CapabilitiesRequired::from_toml_str(manifest_toml) {
        Ok(c) => c,
        Err(e) => {
            let expected_error = case
                .expected_outcome
                .get("error")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("unexpected error: {e}"))?;
            let msg = e.to_string();
            if msg.contains(expected_error) {
                return Ok(());
            }
            return Err(format!(
                "error message mismatch: expected '{expected_error}', got '{msg}'"
            ));
        }
    };

    let scopes = capabilities_required_to_scopes(&caps);
    let expected_scopes = case
        .expected_outcome
        .get("scopes")
        .ok_or("missing expected_outcome.scopes")?;
    let actual_value = serde_json::to_value(&scopes).map_err(|e| e.to_string())?;
    if actual_value != *expected_scopes {
        return Err(format!(
            "scopes mismatch: expected {expected_scopes}, got {actual_value}"
        ));
    }
    Ok(())
}

fn handle_compliance_emit(case: &CaseLine) -> Result<(), String> {
    let envelope_json = case
        .input
        .get("envelope_json")
        .ok_or("missing input.envelope_json")?;

    let result: Result<ComplianceClaimEnvelope, _> = serde_json::from_value(envelope_json.clone());
    match result {
        Ok(envelope) => {
            let expected_ok = case
                .expected_outcome
                .get("ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !expected_ok {
                return Err("expected deserialization error but got Ok".into());
            }
            if envelope.signature.len() != 64 {
                return Err(format!(
                    "signature length mismatch: expected 64, got {}",
                    envelope.signature.len()
                ));
            }
            if envelope.attester_pubkey.len() != 32 {
                return Err(format!(
                    "attester_pubkey length mismatch: expected 32, got {}",
                    envelope.attester_pubkey.len()
                ));
            }
            if let Some(claim_check) = case.expected_outcome.get("claim_check") {
                if let Some(expected_size) =
                    claim_check.get("claim_bytes_min").and_then(|v| v.as_u64())
                {
                    if envelope.claim_bytes.len() < expected_size as usize {
                        return Err(format!(
                            "claim_bytes too small: expected at least {}, got {}",
                            expected_size,
                            envelope.claim_bytes.len()
                        ));
                    }
                }
            }
            Ok(())
        }
        Err(e) => {
            let expected_error = case
                .expected_outcome
                .get("error")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("unexpected deserialize error: {e}"))?;
            let msg = e.to_string();
            if msg.contains(expected_error) {
                Ok(())
            } else {
                Err(format!(
                    "deserialize error mismatch: expected '{expected_error}', got '{msg}'"
                ))
            }
        }
    }
}

fn handle_output_shape(case: &CaseLine) -> Result<(), String> {
    let manifest_toml = case
        .input
        .get("manifest_toml")
        .and_then(|v| v.as_str())
        .ok_or("missing input.manifest_toml")?;

    let shape = match OutputShape::from_toml_str(manifest_toml) {
        Ok(s) => s,
        Err(e) => {
            let expected_error = case
                .expected_outcome
                .get("error")
                .and_then(|v| v.as_str())
                .ok_or_else(|| format!("unexpected error: {e}"))?;
            let msg = e.to_string();
            if msg.contains(expected_error) {
                return Ok(());
            }
            return Err(format!(
                "error message mismatch: expected '{expected_error}', got '{msg}'"
            ));
        }
    };

    let frame_json = case
        .input
        .get("frame_json")
        .ok_or("missing input.frame_json")?;
    let predicate = OutputShapePredicate::from(&shape);
    let result = predicate.check(frame_json);

    if let Some(expected_violation) = case.expected_outcome.get("violation") {
        let expected_type = expected_violation
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or("missing violation.type")?;
        let expected_name = expected_violation
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or("missing violation.name")?;

        let actual = match result {
            Ok(()) => return Err("expected violation but check returned Ok".into()),
            Err(v) => v,
        };

        match (expected_type, &actual) {
            ("MissingField", OutputShapeViolation::MissingField { name })
                if name == expected_name =>
            {
                Ok(())
            }
            ("NullField", OutputShapeViolation::NullField { name }) if name == expected_name => {
                Ok(())
            }
            _ => Err(format!(
                "violation mismatch: expected {expected_type}({expected_name}), got {actual:?}"
            )),
        }
    } else if case
        .expected_outcome
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        result.map_err(|e| format!("unexpected violation: {e:?}"))
    } else {
        Err("unknown expected_outcome shape".into())
    }
}

fn sha256_hex(input: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}
