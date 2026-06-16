---
title: Compliance Claims
sidebar_position: 10
description: Attaching a ComplianceClaimEnvelope to a Spirit for auditability and trust verification.
---

# Compliance Claims

## Problem

Your Spirit operates in an environment that requires verifiable compliance attestations — regulatory audits, trust-tier promotion from `local` to `audited`, or SB-1047 model provenance tracking. You need to attach a signed `ComplianceClaimEnvelope` that the kernel can verify at admission.

## Solution

Build and sign a `ComplianceClaimEnvelope`:

```rust
use maos_spirit_abi::compliance::{
    ComplianceClaimEnvelope, Claim, SigningAlg,
    ExecutionContextFingerprint, TrustTier, SandboxTier,
    PrincipleRef, EvidenceKind, Verdict, Uuid, CapabilityId,
};

/// Build a compliance claim for a Spirit that attests to
/// safety-critical principles with audit evidence.
fn build_compliance_claim() -> ComplianceClaimEnvelope {
    let claim = Claim {
        claim_id: Uuid::from_bytes([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
            0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        ]),
        spirit_class: "my-audited-spirit".into(),
        spirit_version: "1.0.0".into(),
        abi_version: 1,
        attester_id: "auditor@example.com".into(),
        issued_at_epoch_secs: 1718400000, // 2024-06-15T00:00:00Z
        expires_at_epoch_secs: Some(1749936000), // 2025-06-15T00:00:00Z
        execution_context: ExecutionContextFingerprint {
            trust_tier: TrustTier::Audited,
            sandbox_tier: SandboxTier::Baseline,
            capabilities: vec![
                CapabilityId("provider.complete".into()),
            ].into_iter().collect(),
            provider_endpoints: vec![],
            crypto_providers: vec![],
        },
        principles: vec![
            PrincipleRef::I1CapabilityMediation,
            PrincipleRef::I2SandboxIsolation,
            PrincipleRef::I3TransparencyLog,
        ],
        evidence: vec![
            EvidenceKind::TestReport {
                suite_name: "admission-smoke".into(),
                pass_count: 42,
                fail_count: 0,
                report_hash_hex: "a1b2c3d4...".into(),
            },
        ],
        verdict: Verdict::Pass,
    };

    // Serialise the claim to CBOR bytes.
    let claim_bytes = serde_cbor::to_vec(&claim).expect("CBOR encode");

    // Sign the SHA-256 hash of the claim bytes with Ed25519.
    // In production, use a hardware-backed signing key.
    let signature = sign_claim_bytes(&claim_bytes);

    ComplianceClaimEnvelope {
        abi_version: 1,
        signing_alg: SigningAlg::Ed25519,
        signature_bytes: signature,
        signer_public_key: get_public_key(),
        claim_bytes,
    }
}

// Stubs — use a real Ed25519 library (e.g., ed25519-dalek).
fn sign_claim_bytes(data: &[u8]) -> Vec<u8> { vec![0u8; 64] }
fn get_public_key() -> Vec<u8> { vec![0u8; 32] }
```

Reference the claim in the manifest for scheduled invocations or model provenance:

```toml
[class]
name = "my-audited-spirit"
version = "1.0.0"
abi = "1.0"
manifest_schema_version = 3
min_substrate_version = "0.1.0-alpha"
forms = ["rust-inproc"]
trust_tier = "audited"
description = "A Spirit with a compliance claim."

[model_provenance]
covered_model_id = "anthropic.claude-3-opus"
training_data_lineage = ["org.example.dataset-v2"]
last_eval_timestamp = "2026-01-15T00:00:00Z"
```

## Discussion

The `ComplianceClaimEnvelope` is an Ed25519-signed attestation that binds a Spirit class + version to a set of compliance verdicts. The schema is **frozen** at ABI version 1 (Story 1b.4) — changes follow strict ABI-break rules documented in `§8.5`.

**Envelope structure:**

| Field | Purpose |
|---|---|
| `abi_version` | Schema version of the envelope itself |
| `signing_alg` | Algorithm used (`Ed25519`) |
| `signature_bytes` | Ed25519 signature over `sha256(claim_bytes)` |
| `signer_public_key` | Public key for verification |
| `claim_bytes` | CBOR-encoded `Claim` payload |

**The `Claim` payload includes:**

- `principles` — which invariants (I1 through I14) the claim attests compliance with.
- `evidence` — test reports, audit hashes, and other supporting material.
- `verdict` — `Pass`, `Fail`, `Conditional`, or `Withdrawn`.
- `execution_context` — fingerprint of the trust tier, sandbox tier, capabilities, and provider endpoints the claim was evaluated against.

**When you need a compliance claim:**

- Promoting a Spirit from `local` to `audited` trust tier.
- Deploying to environments with regulatory requirements.
- Referencing a claim from `[[schedule]]` entries via `compliance_claim_ref_hex`.
- Satisfying SB-1047 model-provenance requirements (Story 9.4b).

The kernel verifies the signature at admission for `audited`-tier Spirits and logs the result to the transparency log.
