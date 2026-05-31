//! Validation for expanded CCAC items.

use maos_spirit_abi::compliance::ComplianceClaimEnvelope;

use super::CcacItem;
use crate::ValidationOutcome;

/// Validate a single CCAC item: the verdict label is well-formed, the
/// envelope hex decodes back into a `ComplianceClaimEnvelope`, and the
/// rejection metadata is consistent with the verdict.
pub fn validate_item(item: &CcacItem) -> ValidationOutcome {
    if item.expected_verdict != "admit" && item.expected_verdict != "reject" {
        return ValidationOutcome::Invalid {
            reason: format!("bad expected_verdict '{}'", item.expected_verdict),
        };
    }

    // Envelope hex must decode.
    let bytes = match hex::decode(&item.envelope_cbor_hex) {
        Ok(b) => b,
        Err(e) => {
            return ValidationOutcome::Invalid {
                reason: format!("envelope_cbor_hex not hex: {e}"),
            }
        }
    };
    if serde_cbor::from_slice::<ComplianceClaimEnvelope>(&bytes).is_err() {
        return ValidationOutcome::Invalid {
            reason: "envelope_cbor_hex does not decode to ComplianceClaimEnvelope".into(),
        };
    }

    match item.expected_verdict.as_str() {
        "admit" => {
            if item.expected_rejection_kind.is_some() {
                return ValidationOutcome::Invalid {
                    reason: "admit item carries a rejection kind".into(),
                };
            }
        }
        "reject" => {
            let kind = match item.expected_rejection_kind.as_deref() {
                Some(k) => k,
                None => {
                    return ValidationOutcome::Invalid {
                        reason: "reject item missing rejection kind".into(),
                    }
                }
            };
            // ContextDrift items MUST name the drifted field.
            if kind == "ContextDrift" && item.expected_rejection_field.is_none() {
                return ValidationOutcome::Invalid {
                    reason: "ContextDrift item missing expected_rejection_field".into(),
                };
            }
        }
        _ => unreachable!(),
    }

    ValidationOutcome::Valid
}
