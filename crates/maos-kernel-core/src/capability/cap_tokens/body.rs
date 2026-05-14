#![forbid(unsafe_code)]

//! Capability token body — the canonical byte-stream that gets signed.

use maos_domain::invariants::i1::{IntentClass, Scope, TokenId};

/// The token body that gets signed by `CryptoProvider::sign_capability_token`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapTokenBody {
    pub token_id: TokenId,
    pub spirit_pid: u32,
    pub boot_nonce: u64,
    pub expiry_ns: u64,
    pub scope_hash: [u8; 32],
    pub posture_snapshot_hash: [u8; 32],
    pub intent_class: IntentClass,
}

impl CapTokenBody {
    /// Produce the canonical byte-stream for signing.
    pub fn to_signing_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128);
        out.extend_from_slice(&self.token_id.0);
        out.extend_from_slice(&self.spirit_pid.to_le_bytes());
        out.extend_from_slice(&self.boot_nonce.to_le_bytes());
        out.extend_from_slice(&self.expiry_ns.to_le_bytes());
        out.extend_from_slice(&self.scope_hash);
        out.extend_from_slice(&self.posture_snapshot_hash);
        // IntentClass discriminant as single byte
        out.push(intent_class_discriminant(&self.intent_class));
        out
    }
}

fn intent_class_discriminant(ic: &IntentClass) -> u8 {
    match ic {
        IntentClass::HighPrivilege => 0,
        IntentClass::Standard => 1,
        IntentClass::Readonly => 2,
    }
}

/// Hash a scope to a 32-byte digest using SHA-256.
pub fn scope_hash(scope: &Scope) -> [u8; 32] {
    use ring::digest;
    let mut ctx = digest::Context::new(&digest::SHA256);
    let bytes = serde_json::to_vec(scope).expect("Scope serialization is infallible for the nine v0.1-β variants");
    ctx.update(&bytes);
    let d = ctx.finish();
    let mut out = [0u8; 32];
    out.copy_from_slice(d.as_ref());
    out
}
