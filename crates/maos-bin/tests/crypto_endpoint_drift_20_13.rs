#![cfg(feature = "network")]

#[test]
fn empty_url_does_not_unpin_provider_or_model() {
    use maos_compliance::builder::{build_signed_envelope, encode_claim_bytes};
    use maos_compliance::evaluator::{DriftField, EComplianceRejection};
    use maos_compliance::{evaluate_envelope_with, ComplianceVerdict, RuntimeExecutionContext};
    use maos_domain::ports::registry::TrustTier;
    use maos_spirit_abi::compliance::{
        CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    };
    use ring::signature::{Ed25519KeyPair, KeyPair};
    use std::collections::BTreeSet;
    let key = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
    let public: [u8; 32] = key.public_key().as_ref().try_into().unwrap();
    let runtime = ExecutionContextFingerprint {
        manifest_hash: [1; 32],
        spirit_version: "1".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: BTreeSet::new(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: "provider".into(),
            endpoint_url: "https://example.org".into(),
            model_id: Some("model".into()),
        },
        crypto_provider: CryptoProviderId("ring".into()),
    };
    let ctx = RuntimeExecutionContext {
        manifest_hash: runtime.manifest_hash,
        spirit_version: runtime.spirit_version.clone(),
        effective_trust_tier: runtime.trust_tier,
        effective_sandbox_tier: runtime.sandbox_tier,
        runtime_provider_endpoint: runtime.provider_endpoint.clone(),
        runtime_crypto_provider: runtime.crypto_provider.clone(),
        capability_scope: runtime.capability_scope.clone(),
    };
    for change_provider in [true, false] {
        let mut claimed = runtime.clone();
        claimed.provider_endpoint.endpoint_url.clear();
        if change_provider {
            claimed.provider_endpoint.provider_id = "other".into();
        } else {
            claimed.provider_endpoint.model_id = Some("other".into());
        }
        let claim = build_signed_envelope(encode_claim_bytes(&claimed, None), &key, public);
        let verdict = evaluate_envelope_with(
            &claim,
            &ctx,
            &maos_kernel_core::security::crypto::RingCryptoProvider,
        );
        assert!(
            matches!(
                verdict,
                ComplianceVerdict::Reject(EComplianceRejection::ContextDrift {
                    field: DriftField::ProviderEndpoint,
                    ..
                })
            ),
            "{verdict:?}"
        );
    }
}
