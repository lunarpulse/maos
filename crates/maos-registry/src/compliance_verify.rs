//! ComplianceClaim structural verification — **LIFTED to `maos-compliance`** (Story 7.3).
//!
//! Story 5.5d shipped the v0.5-α structural verifier here. Story 7.3 promotes
//! it to the v1.0-binding semantic evaluator and moves the implementation to
//! the single-home crate `maos-compliance`. This module is now a thin
//! backward-compat shim: it RE-EXPORTS the lifted helpers and DELEGATES
//! `verify_envelope_structural` to `maos_compliance::evaluator` — there is no
//! second copy of the verification logic in the workspace.
//!
//! New code should call `maos_compliance::evaluate_envelope` with a
//! `RuntimeExecutionContext` directly (see `admission.rs`). This shim exists so
//! `maos-spirit-cli` (the producer) and the legacy fixtures keep compiling.

use maos_domain::ports::registry::SignedPackage;
use maos_spirit_abi::compliance::ComplianceClaimEnvelope;

// Re-export the lifted helpers from their single home so existing imports
// (`maos_registry::compliance_verify::{compute_fingerprint_hash,
// extract_manifest_fingerprint_fields, ManifestFingerprintFields}`) keep working.
pub use maos_compliance::canonical_cbor::compute_fingerprint_hash;
pub use maos_compliance::runtime_context::{
    extract_manifest_fingerprint_fields, ManifestFingerprintFields,
};

use maos_compliance::evaluator::{evaluate_envelope_at, ComplianceVerdict, EComplianceRejection};
use maos_compliance::RuntimeExecutionContext;

/// Result of structural envelope verification (v0.5-α compat shape).
#[derive(Debug)]
pub enum VerificationResult {
    /// Envelope is structurally valid.
    Ok,
    /// Envelope signature does not verify.
    SignatureInvalid,
    /// Envelope claim_bytes cannot be decoded.
    ClaimDecodeFailure(String),
    /// Fingerprint hash / structural-field mismatch.
    Drift {
        actual_hex: String,
        claimed_hex: String,
    },
}

/// Verify a ComplianceClaim envelope against the MANIFEST-derived execution
/// context (the v0.5-α structural semantics).
///
/// DELEGATES to `maos_compliance::evaluate_envelope` — the runtime context is
/// built entirely from the manifest, so this reproduces the legacy
/// manifest-only structural floor. The v1.0 RUNTIME-context comparison lives in
/// `admission.rs`, which builds a `RuntimeExecutionContext` from the admission
/// decision + composition root rather than the manifest alone.
pub fn verify_envelope_structural(
    envelope: &ComplianceClaimEnvelope,
    pkg: &SignedPackage,
) -> VerificationResult {
    let fields = extract_manifest_fingerprint_fields(&pkg.manifest_toml);
    let ctx = RuntimeExecutionContext::from_admission(
        fields.trust_tier,
        fields.sandbox_tier,
        pkg,
        &fields.provider_endpoint,
        &fields.crypto_provider,
    );
    // now = 0 → the expiry guard (`now > expires_at`) never fires at the
    // v0.5-α structural layer.
    match evaluate_envelope_at(envelope, &ctx, 0) {
        ComplianceVerdict::Admit => VerificationResult::Ok,
        ComplianceVerdict::Reject(EComplianceRejection::SignatureInvalid) => {
            VerificationResult::SignatureInvalid
        }
        ComplianceVerdict::Reject(EComplianceRejection::MalformedClaim(m)) => {
            VerificationResult::ClaimDecodeFailure(m)
        }
        ComplianceVerdict::Reject(EComplianceRejection::ContextDrift {
            actual, claimed, ..
        }) => VerificationResult::Drift {
            actual_hex: actual,
            claimed_hex: claimed,
        },
        ComplianceVerdict::Reject(EComplianceRejection::ExpiredClaim { .. }) => {
            VerificationResult::ClaimDecodeFailure("claim expired".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_spirit_abi::compliance::{ComplianceClaimEnvelope, SigningAlg};

    fn empty_envelope() -> ComplianceClaimEnvelope {
        ComplianceClaimEnvelope {
            signature: [0u8; 64],
            attester_pubkey: [1u8; 32],
            claim_bytes: vec![],
            signing_alg: SigningAlg::Ed25519,
        }
    }

    fn test_pkg() -> SignedPackage {
        SignedPackage::new(
            maos_domain::ports::registry::SpiritId::from("test"),
            "0.1.0".into(),
            b"[manifest]".to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            [0xBBu8; 32],
            empty_envelope(),
        )
    }

    #[test]
    fn tampered_signature_is_rejected_via_delegation() {
        let pkg = test_pkg();
        let mut envelope = empty_envelope();
        envelope.signature = [0u8; 64];
        envelope.claim_bytes = b"hello".to_vec();
        let result = verify_envelope_structural(&envelope, &pkg);
        assert!(matches!(result, VerificationResult::SignatureInvalid));
    }

    #[test]
    fn fingerprint_mismatch_is_rejected_via_delegation() {
        // Real key, valid sig over a claim whose structural fields drift from
        // the manifest → Drift (delegated to maos-compliance evaluator).
        use maos_compliance::builder::build_signed_envelope;
        #[allow(deprecated)]
        use maos_compliance::builder::seeded_keypair;
        let (kp, pk) = seeded_keypair(0x150C_04A5);
        let manifest = b"[spirit]\nname = \"test\"\nversion = \"0.1.0\"\ntrust_tier = \"local\"\nsandbox_tier = \"t0\"\n";
        let claim = serde_json::json!({
            "fingerprint_hash": hex::encode([0u8;32]),
            "trust_tier": "public_untrusted", // drifts from manifest "local"
            "sandbox_tier": "t3",
            "capability_scope": [],
            "provider_endpoint": {"provider_id": "", "endpoint_url": ""},
            "crypto_provider": ""
        });
        let claim_bytes = serde_json::to_vec(&claim).unwrap();
        let envelope = build_signed_envelope(claim_bytes, &kp, pk);
        let pkg = SignedPackage::new(
            maos_domain::ports::registry::SpiritId::from("test"),
            "0.1.0".into(),
            manifest.to_vec(),
            b"binary".to_vec(),
            [0xAAu8; 64],
            pk,
            empty_envelope(),
        );
        assert!(matches!(
            verify_envelope_structural(&envelope, &pkg),
            VerificationResult::Drift { .. }
        ));
    }
}
