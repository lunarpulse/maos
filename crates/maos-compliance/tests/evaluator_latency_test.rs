//! Story 7.3 AC2 item 6 — evaluator latency budget: P99 < 10ms over N=1000.
//!
//! Wall-clock micro-bench (criterion-free). A fixed pre-built envelope + ctx;
//! no I/O in the loop. The evaluator does one Ed25519 verify + one CBOR decode
//! + one CBOR encode/SHA-256, all of which are sub-millisecond, so the budget
//! is generous — the gate guards against an accidental O(n) regression.

#![allow(deprecated)]

use std::collections::BTreeSet;
use std::time::Instant;

use maos_compliance::builder::build_self_attested_envelope;
#[allow(deprecated)]
use maos_compliance::builder::seeded_keypair;
use maos_compliance::evaluator::evaluate_envelope_at;
use maos_compliance::{ComplianceVerdict, RuntimeExecutionContext};
use maos_spirit_abi::compliance::{
    CapabilityId, CryptoProviderId, ExecutionContextFingerprint, ProviderEndpointPin, SandboxTier,
    TrustTier,
};

#[test]
fn evaluator_p99_under_10ms() {
    let mut caps = BTreeSet::new();
    caps.insert(CapabilityId("fs.read".into()));
    caps.insert(CapabilityId("net.connect".into()));
    let fp = ExecutionContextFingerprint {
        manifest_hash: [5u8; 32],
        spirit_version: "1.0.0".into(),
        trust_tier: TrustTier::PublicUntrusted,
        sandbox_tier: SandboxTier::T3,
        capability_scope: caps.clone(),
        provider_endpoint: ProviderEndpointPin {
            provider_id: "anthropic".into(),
            endpoint_url: "https://api.anthropic.com".into(),
            model_id: Some("claude".into()),
        },
        crypto_provider: CryptoProviderId("ring".into()),
    };
    let (kp, pk) = seeded_keypair(0x1A7E_0C99);
    let env = build_self_attested_envelope(&fp, &kp, pk);
    let ctx = RuntimeExecutionContext {
        manifest_hash: fp.manifest_hash,
        spirit_version: fp.spirit_version.clone(),
        effective_trust_tier: fp.trust_tier,
        effective_sandbox_tier: fp.sandbox_tier,
        runtime_provider_endpoint: fp.provider_endpoint.clone(),
        runtime_crypto_provider: fp.crypto_provider.clone(),
        capability_scope: caps,
    };

    const N: usize = 1000;
    let now = 1_900_000_000_000u64;
    let mut durations = Vec::with_capacity(N);
    for _ in 0..N {
        let t = Instant::now();
        let v = evaluate_envelope_at(&env, &ctx, now);
        durations.push(t.elapsed());
        assert_eq!(v, ComplianceVerdict::Admit);
    }
    durations.sort();
    let p99 = durations[(N as f64 * 0.99) as usize - 1];
    let p99_ms = p99.as_secs_f64() * 1000.0;
    assert!(
        p99_ms < 10.0,
        "evaluator P99 {p99_ms:.3}ms exceeds 10ms budget"
    );
}
