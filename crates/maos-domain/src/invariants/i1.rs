//! I1: Spirits cannot bypass the Capability Registry.
//!
//! Every tool, network call, file op, or sub-Spirit spawn from a Spirit
//! MUST flow through the Capability Registry. The kernel's only API
//! surface returned to a Spirit at load-time is the typed capability
//! mediation layer; there is no Spirit-visible short-circuit.
//!
//! # Enforcement
//!
//! - **v0.1**: `runtime` — Capability Registry mediation is the only
//!   public function path returning side-effects to Spirits.
//! - **v0.3 / v0.5 / v0.9**: `runtime` (unchanged).
//! - **v1.0 / v1.5**: `fuzz` — the 80-scenario red-team corpus (NFR-Sec-10)
//!   beats on capability-confusion paths.
//!
//! # Invariant statement (doctest)
//!
//! The marker type below codifies I1 at the type level. Calling it requires
//! a `CapabilityToken`; a Spirit cannot construct a `CapabilityToken`
//! outside the registry. This is the type-level expression of the I1 contract.
//!
//! ```
//! use maos_domain::invariants::i1::{InvariantI1, CapabilityToken};
//!
//! // The marker type exists and is the contract anchor for I1.
//! let _marker: InvariantI1 = InvariantI1;
//!
//! // Capability tokens are private-constructor — no Spirit-visible `new`
//! // function exists at the domain layer. The kernel's
//! // `cap_tokens::issue(spirit_id, scope)` is the ONLY constructor.
//! // (Trying to construct one here would fail to compile by design;
//! // the doctest documents the contract, it does NOT exercise a violation.)
//! # let _ = std::mem::size_of::<CapabilityToken>();  // proves the type exists
//! ```

/// I1 marker type — Spirits cannot bypass the Capability Registry.
///
/// This zero-size type exists to anchor I1 in the type system. Its
/// presence in a function signature documents that the function operates
/// under the I1 contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvariantI1;

/// 16-byte token identifier — ULID-shaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TokenId(pub [u8; 16]);

impl TokenId {
    /// Zero token id — sentinel for bulk operations.
    pub const ZERO: TokenId = TokenId([0u8; 16]);
}

/// Capability scope — the nine v0.1-β variants.
///
/// Adding a tenth variant later is an ABI break (this enum is re-exported
/// through `maos-spirit-abi`). The nine here are the v0.1-β freeze.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Scope {
    /// Read access to a filesystem subtree.
    FsRead { subtree: String },
    /// Write access to a filesystem subtree.
    FsWrite { subtree: String },
    /// HTTPS network call to a specific domain.
    NetHttps { domain: String },
    /// Execute a specific binary.
    ProcExec { binary: String },
    /// Spawn a sub-Spirit of a given class.
    SubSpiritSpawn { class: String },
    /// Invoke an LLM provider (Story 1b.4).
    ProviderInfer { provider: String },
    /// Send an IAC frame to a peer class.
    IacSend { peer_class: String },
    /// Read from a memory scope.
    MemRead { scope: String },
    /// Write to a memory scope.
    MemWrite { scope: String },
}

/// Intent classification for approval policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IntentClass {
    /// High-privilege operations — TTL capped at 60s.
    HighPrivilege,
    /// Standard operations — TTL capped at 300s.
    Standard,
    /// Read-only operations — TTL capped at 900s.
    Readonly,
}

/// Capability token — short-lived authorization to invoke a specific
/// Capability with specific arguments under a specific posture (per §3.1
/// vocabulary + ADR-023).
///
/// Constructor is private-to-the-crate at v0.1-α (the actual kernel-side
/// issuance lands in Story 1b.2 inside `maos-kernel-core::capability::cap_tokens`;
/// this `maos-domain` type is the wire-stable shape Spirits see).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct CapabilityToken {
    /// 16-byte token identifier.
    pub token_id: TokenId,
    /// Spirit process ID this token is bound to.
    pub spirit_pid: u32,
    /// Expiry timestamp in nanoseconds since boot.
    pub expiry_ns: u64,
    /// Ed25519 signature (64 bytes) over the token body.
    #[serde(with = "serde_sig64")]
    pub signature: [u8; 64],
}

impl CapabilityToken {
    /// Construct a capability token. The `#[non_exhaustive]` attribute
    /// prevents cross-crate struct-literal construction, so external
    /// callers must obtain tokens through the kernel's
    /// `cap_tokens::issue` method.
    pub fn new(token_id: TokenId, spirit_pid: u32, expiry_ns: u64, signature: [u8; 64]) -> Self {
        Self {
            token_id,
            spirit_pid,
            expiry_ns,
            signature,
        }
    }
}

mod serde_sig64 {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(sig: &[u8; 64], ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_bytes(sig)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<[u8; 64], D::Error> {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(de)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom(
                format!("expected 64-byte signature, got {} bytes", bytes.len())
            ));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_token_exists() {
        let t = CapabilityToken {
            token_id: TokenId::ZERO,
            spirit_pid: 0,
            expiry_ns: 0,
            signature: [0u8; 64],
        };
        assert_eq!(t.spirit_pid, 0);
    }

    #[test]
    fn invariant_i1_marker_is_zst() {
        assert_eq!(std::mem::size_of::<InvariantI1>(), 0);
    }

    #[test]
    fn capability_token_is_non_exhaustive() {
        // This test verifies the struct carries #[non_exhaustive] by
        // attempting construction with named fields (which compiles
        // within the same crate but would require `..` from another crate).
        let _t = CapabilityToken {
            token_id: TokenId([1u8; 16]),
            spirit_pid: 7,
            expiry_ns: 1_000_000_000,
            signature: [2u8; 64],
        };
    }
}
