#![forbid(unsafe_code)]

//! Structural validator for the frozen ComplianceClaim schema (AC1).
//!
//! This module performs **structural** validation only — no cryptographic
//! signature verification. Signature verification (`CryptoProvider::verify_signature`)
//! and the §8.5 context-drift admission check are deferred to Epic 7
//! (Story 7.3 per `compliance.rs` doc-comment).
//!
//! The validator always returns a `Result` — never panics, never silently
//! passes. This is the "100% schema validation and 100% emit-rate" contract.

use maos_spirit_abi::compliance::{Claim, ComplianceClaimEnvelope, PrincipleRef, Verdict};

/// A claim that has passed structural validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedClaim {
    /// The decoded claim payload.
    pub claim: Claim,
}

/// Structural validation error — concrete named variants, no blanket `#[from]`.
#[derive(Debug, thiserror::Error)]
pub enum ComplianceValidationError {
    /// `claim_bytes` is empty.
    #[error("claim_bytes is empty")]
    EmptyClaimBytes,
    /// `signature` is all-zero (the type guarantees 64 bytes; this checks
    /// the *meaningful* non-zero requirement per DW1).
    #[error("signature is all-zero")]
    AllZeroSignature,
    /// `claim_bytes` could not be decoded as CBOR.
    #[error("malformed CBOR: {0}")]
    MalformedCbor(String),
    /// Decoded claim does not round-trip to identical canonical CBOR bytes.
    #[error("non-canonical encoding: re-encoded bytes differ from claim_bytes")]
    NonCanonicalEncoding,
    /// The claim carries an unknown enum variant that this validator version
    /// does not recognize (fail-closed: newer-schema claims rejected).
    #[error("unknown enum variant in field '{field}': {value}")]
    UnknownEnumVariant { field: String, value: String },
    /// `expires_at_unix_ms` is not greater than `issued_at_unix_ms`.
    #[error("expiry {expiry} is not after issue {issue}")]
    ExpiryBeforeIssue { issue: u64, expiry: u64 },
    /// `issued_at_unix_ms` is zero.
    #[error("issued_at_unix_ms is zero")]
    ZeroIssueTimestamp,
}

/// Validate a `ComplianceClaimEnvelope` structurally (no crypto).
///
/// Checks performed:
/// 1. `claim_bytes` non-empty.
/// 2. `signature` not all-zero.
/// 3. `claim_bytes` decodes as canonical CBOR into `Claim`.
/// 4. Re-encoded `Claim` is byte-identical to `claim_bytes` (canonical round-trip).
/// 5. No `UnknownPrinciple` or `UnknownVerdict` variants (fail-closed).
/// 6. `issued_at_unix_ms` non-zero.
/// 7. If `expires_at_unix_ms` is `Some`, it is `> issued_at_unix_ms`.
pub fn validate_envelope(
    env: &ComplianceClaimEnvelope,
) -> Result<ValidatedClaim, ComplianceValidationError> {
    // 1. Non-empty claim_bytes
    if env.claim_bytes.is_empty() {
        return Err(ComplianceValidationError::EmptyClaimBytes);
    }

    // 2. Signature not all-zero
    if env.signature.iter().all(|b| *b == 0) {
        return Err(ComplianceValidationError::AllZeroSignature);
    }

    // 3. CBOR decode
    let claim: Claim = ciborium::de::from_reader(&env.claim_bytes[..])
        .map_err(|e| ComplianceValidationError::MalformedCbor(format!("{e}")))?;

    // 4. Canonical round-trip
    let mut re_encoded = Vec::new();
    ciborium::ser::into_writer(&claim, &mut re_encoded)
        .map_err(|e| ComplianceValidationError::MalformedCbor(format!("re-encode failed: {e}")))?;
    if re_encoded != env.claim_bytes {
        return Err(ComplianceValidationError::NonCanonicalEncoding);
    }

    // 5. No Unknown* variants (fail-closed)
    for (idx, pr) in claim.principle_refs.iter().enumerate() {
        if matches!(pr, PrincipleRef::UnknownPrinciple) {
            return Err(ComplianceValidationError::UnknownEnumVariant {
                field: format!("principle_refs[{idx}]"),
                value: "UnknownPrinciple".into(),
            });
        }
    }
    // EvidenceKind has no Unknown fallback variant today; when one is added
    // (ABI-safe per row #8), a fail-closed check belongs here.
    let _ = &claim.evidence;
    if matches!(claim.verdict, Verdict::UnknownVerdict) {
        return Err(ComplianceValidationError::UnknownEnumVariant {
            field: "verdict".into(),
            value: "UnknownVerdict".into(),
        });
    }

    // 6. Timestamp sanity: issued_at non-zero
    if claim.issued_at_unix_ms == 0 {
        return Err(ComplianceValidationError::ZeroIssueTimestamp);
    }

    // 7. Expiry > issue (if present)
    if let Some(expiry) = claim.expires_at_unix_ms {
        if expiry <= claim.issued_at_unix_ms {
            return Err(ComplianceValidationError::ExpiryBeforeIssue {
                issue: claim.issued_at_unix_ms,
                expiry,
            });
        }
    }

    Ok(ValidatedClaim { claim })
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{
        Claim, ComplianceClaimEnvelope, EvidenceKind, PrincipleRef, ProviderEndpointPin,
        SandboxTier, SigningAlg, TrustTier, Uuid, Verdict,
    };

    fn minimal_claim() -> Claim {
        Claim {
            claim_id: Uuid::from_bytes([0u8; 16]),
            issued_at_unix_ms: 1_700_000_000_000,
            expires_at_unix_ms: None,
            principle_refs: vec![PrincipleRef::Hipaa164308],
            evidence: vec![],
            verdict: Verdict::Admit,
        }
    }

    fn minimal_envelope() -> ComplianceClaimEnvelope {
        let claim = minimal_claim();
        let mut claim_bytes = Vec::new();
        ciborium::ser::into_writer(&claim, &mut claim_bytes).unwrap();
        ComplianceClaimEnvelope {
            signature: [1u8; 64],
            attester_pubkey: [2u8; 32],
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        }
    }

    #[test]
    fn well_formed_claim_ok() {
        let env = minimal_envelope();
        let result = validate_envelope(&env);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        assert_eq!(result.unwrap().claim.claim_id.as_bytes(), &[0u8; 16]);
    }

    #[test]
    fn empty_claim_bytes_err() {
        let mut env = minimal_envelope();
        env.claim_bytes.clear();
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(err, ComplianceValidationError::EmptyClaimBytes));
    }

    #[test]
    fn all_zero_signature_err() {
        let mut env = minimal_envelope();
        env.signature = [0u8; 64];
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(err, ComplianceValidationError::AllZeroSignature));
    }

    #[test]
    fn malformed_cbor_err() {
        let mut env = minimal_envelope();
        env.claim_bytes = vec![0xFF, 0xFF];
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(err, ComplianceValidationError::MalformedCbor(_)));
    }

    #[test]
    fn non_canonical_encoding_err() {
        let mut env = minimal_envelope();
        // Append a trailing byte to make re-encode differ
        env.claim_bytes.push(0x00);
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(
            err,
            ComplianceValidationError::NonCanonicalEncoding
        ));
    }

    #[test]
    fn unknown_principle_err() {
        let mut claim = minimal_claim();
        claim.principle_refs = vec![PrincipleRef::UnknownPrinciple];
        let mut claim_bytes = Vec::new();
        ciborium::ser::into_writer(&claim, &mut claim_bytes).unwrap();
        let env = ComplianceClaimEnvelope {
            signature: [1u8; 64],
            attester_pubkey: [2u8; 32],
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };
        let err = validate_envelope(&env).unwrap_err();
        assert!(
            matches!(err, ComplianceValidationError::UnknownEnumVariant { field, value } if field.starts_with("principle_refs") && value == "UnknownPrinciple")
        );
    }

    #[test]
    fn unknown_verdict_err() {
        let mut claim = minimal_claim();
        claim.verdict = Verdict::UnknownVerdict;
        let mut claim_bytes = Vec::new();
        ciborium::ser::into_writer(&claim, &mut claim_bytes).unwrap();
        let env = ComplianceClaimEnvelope {
            signature: [1u8; 64],
            attester_pubkey: [2u8; 32],
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };
        let err = validate_envelope(&env).unwrap_err();
        assert!(
            matches!(err, ComplianceValidationError::UnknownEnumVariant { field, value } if field == "verdict" && value == "UnknownVerdict")
        );
    }

    #[test]
    fn zero_issue_timestamp_err() {
        let mut claim = minimal_claim();
        claim.issued_at_unix_ms = 0;
        let mut claim_bytes = Vec::new();
        ciborium::ser::into_writer(&claim, &mut claim_bytes).unwrap();
        let env = ComplianceClaimEnvelope {
            signature: [1u8; 64],
            attester_pubkey: [2u8; 32],
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(err, ComplianceValidationError::ZeroIssueTimestamp));
    }

    #[test]
    fn expiry_before_issue_err() {
        let mut claim = minimal_claim();
        claim.issued_at_unix_ms = 1000;
        claim.expires_at_unix_ms = Some(500);
        let mut claim_bytes = Vec::new();
        ciborium::ser::into_writer(&claim, &mut claim_bytes).unwrap();
        let env = ComplianceClaimEnvelope {
            signature: [1u8; 64],
            attester_pubkey: [2u8; 32],
            claim_bytes,
            signing_alg: SigningAlg::Ed25519,
        };
        let err = validate_envelope(&env).unwrap_err();
        assert!(matches!(
            err,
            ComplianceValidationError::ExpiryBeforeIssue {
                issue: 1000,
                expiry: 500
            }
        ));
    }

    #[test]
    fn every_malformation_has_distinct_error_variant() {
        // This test asserts the "100% emit-rate" contract: every malformation
        // class maps to its own distinct error variant.
        let errors = vec![
            validate_envelope(&{
                let mut e = minimal_envelope();
                e.claim_bytes.clear();
                e
            }),
            validate_envelope(&{
                let mut e = minimal_envelope();
                e.signature = [0u8; 64];
                e
            }),
            validate_envelope(&{
                let mut e = minimal_envelope();
                e.claim_bytes = vec![0xFF];
                e
            }),
            validate_envelope(&{
                let mut e = minimal_envelope();
                e.claim_bytes.push(0x00);
                e
            }),
            validate_envelope(&{
                let mut c = minimal_claim();
                c.principle_refs = vec![PrincipleRef::UnknownPrinciple];
                let mut b = Vec::new();
                ciborium::ser::into_writer(&c, &mut b).unwrap();
                let mut e = minimal_envelope();
                e.claim_bytes = b;
                e
            }),
            validate_envelope(&{
                let mut c = minimal_claim();
                c.verdict = Verdict::UnknownVerdict;
                let mut b = Vec::new();
                ciborium::ser::into_writer(&c, &mut b).unwrap();
                let mut e = minimal_envelope();
                e.claim_bytes = b;
                e
            }),
            validate_envelope(&{
                let mut c = minimal_claim();
                c.issued_at_unix_ms = 0;
                let mut b = Vec::new();
                ciborium::ser::into_writer(&c, &mut b).unwrap();
                let mut e = minimal_envelope();
                e.claim_bytes = b;
                e
            }),
            validate_envelope(&{
                let mut c = minimal_claim();
                c.issued_at_unix_ms = 1000;
                c.expires_at_unix_ms = Some(500);
                let mut b = Vec::new();
                ciborium::ser::into_writer(&c, &mut b).unwrap();
                let mut e = minimal_envelope();
                e.claim_bytes = b;
                e
            }),
        ];
        let variant_names: Vec<String> = errors
            .iter()
            .map(|r| match r {
                Ok(_) => "Ok".into(),
                Err(e) => format!("{e:?}").split('(').next().unwrap().to_string(),
            })
            .collect();
        let unique: std::collections::HashSet<_> = variant_names.iter().collect();
        assert_eq!(
            unique.len(),
            variant_names.len(),
            "expected all malformation classes to produce distinct error variants, got: {variant_names:?}"
        );
    }
}
