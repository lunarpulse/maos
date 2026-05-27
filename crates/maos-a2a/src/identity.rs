//! Peer identity types for A2A.
//!
//! Per architecture §7.2: "Each Host's deployment configuration names the
//! other Host's mTLS certificate fingerprint. There is no discovery protocol
//! because there is nothing to discover — the operator names the two
//! endpoints."

use serde::{Deserialize, Serialize};
use std::fmt;

/// Opaque peer identity — operator-declared in config.
///
/// Two construction surfaces:
/// 1. Free-form operator-declared id (`PeerId::from_str("host-b-prod-edge")`)
/// 2. Derived from cert fingerprint via `PeerId::from_fingerprint`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PeerId(String);

impl PeerId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn from_fingerprint(fp: &PeerCertFingerprint) -> Self {
        Self(format!("peer-{}", fp.short()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// SHA-256 cert fingerprint per architecture §7.2.
///
/// Wire form is `"sha256:<hex64>"`. The bytes form is the canonical
/// 32-byte digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerCertFingerprint {
    /// Algorithm tag — always `"sha256"` at v0.5; reserved for future agility.
    #[serde(default = "default_algo")]
    pub algo: String,
    /// Hex-encoded fingerprint bytes (lowercase, no separators).
    pub hex: String,
}

fn default_algo() -> String {
    "sha256".to_string()
}

impl PeerCertFingerprint {
    /// Compute from a DER-encoded cert via SHA-256.
    pub fn from_cert_der(der: &[u8]) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(der);
        Self {
            algo: "sha256".to_string(),
            hex: hex::encode(digest),
        }
    }

    /// Parse the wire form `"sha256:<hex64>"`. Returns `None` for malformed input.
    pub fn parse(s: &str) -> Option<Self> {
        let (algo, hex_part) = s.split_once(':')?;
        if algo != "sha256" {
            return None;
        }
        if hex_part.len() != 64 || !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self {
            algo: algo.to_string(),
            hex: hex_part.to_lowercase(),
        })
    }

    /// Short form for human display — first 8 hex chars.
    pub fn short(&self) -> String {
        self.hex.chars().take(8).collect()
    }

    /// Wire form `"sha256:<hex64>"`.
    pub fn wire(&self) -> String {
        format!("{}:{}", self.algo, self.hex)
    }
}

impl fmt::Display for PeerCertFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.wire())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_from_cert_der_round_trip() {
        let der = b"fake-cert-bytes";
        let fp = PeerCertFingerprint::from_cert_der(der);
        assert_eq!(fp.algo, "sha256");
        assert_eq!(fp.hex.len(), 64);
        // Idempotent
        let fp2 = PeerCertFingerprint::from_cert_der(der);
        assert_eq!(fp, fp2);
    }

    #[test]
    fn fingerprint_parse_round_trip() {
        let wire = "sha256:abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab";
        let fp = PeerCertFingerprint::parse(wire).expect("valid wire form");
        assert_eq!(fp.wire(), wire);
    }

    #[test]
    fn fingerprint_parse_rejects_wrong_algo() {
        assert!(PeerCertFingerprint::parse("md5:abcd").is_none());
    }

    #[test]
    fn fingerprint_parse_rejects_short_hex() {
        assert!(PeerCertFingerprint::parse("sha256:abcd").is_none());
    }

    #[test]
    fn fingerprint_parse_rejects_non_hex() {
        // 64 chars but 'z' is non-hex
        let bad = format!("sha256:{}", "z".repeat(64));
        assert!(PeerCertFingerprint::parse(&bad).is_none());
    }

    #[test]
    fn peer_id_display_round_trip() {
        let p = PeerId::new("host-b");
        assert_eq!(p.to_string(), "host-b");
    }

    #[test]
    fn peer_id_from_fingerprint() {
        let der = b"x";
        let fp = PeerCertFingerprint::from_cert_der(der);
        let pid = PeerId::from_fingerprint(&fp);
        assert!(pid.as_str().starts_with("peer-"));
    }
}
