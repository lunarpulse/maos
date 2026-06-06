//! Operator A2A configuration per architecture §7.2 + ADR-003.
//!
//! Schema lives under the daemon config file's `[[a2a.peer]]` sections.
//! Per `[[feedback_mechanical_gates_compound_promises_decay]]` we use
//! `#[serde(deny_unknown_fields)]` to prevent silent acceptance of operator
//! typos.

use crate::consent::ConsentAllowlists;
use crate::identity::{PeerCertFingerprint, PeerId};
use serde::{Deserialize, Serialize};

/// A2A deployment profile.
///
/// `Loopback` — `127.0.0.1`-bound endpoints with self-signed mTLS (FR23a v0.8).
/// `CrossHost` — operator-managed PKI + JSON-RPC over mTLS/TCP (FR23b v1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum A2AProfile {
    Loopback,
    CrossHost,
}

/// Operator A2A config — top-level `[a2a]` table with one or more
/// `[[a2a.peer]]` sections.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct A2AConfig {
    #[serde(default)]
    pub peer: Vec<A2APeerConfig>,
}

/// Per-peer A2A configuration.
///
/// Operator declares each peer by `peer_id`, transport endpoint, expected
/// cert fingerprint, deployment profile, and ADR-012 send/accept allowlists.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct A2APeerConfig {
    /// Operator-declared peer identity (e.g. `"host-b-prod-edge"`).
    pub peer_id: PeerId,

    /// Transport endpoint URL (e.g. `"tls://host-b.internal:7443"` for
    /// cross-Host or `"tls://127.0.0.1:7443"` for loopback). The scheme MUST
    /// be `tls://` at v0.5; the adapter rejects other schemes.
    pub endpoint: String,

    /// Expected mTLS cert fingerprint per architecture §7.2. First-contact
    /// pin records the OBSERVED fingerprint and asserts it equals this
    /// declared fingerprint; mismatch fires `EPinMismatch::Mismatch` at
    /// first-contact.
    pub cert_fingerprint: PeerCertFingerprint,

    /// Deployment profile.
    #[serde(default = "default_profile")]
    pub profile: A2AProfile,

    /// ADR-012 per-direction send/accept allowlists. Empty allowlists deny
    /// all intents (default-deny semantics).
    #[serde(default, flatten)]
    pub allowlists: ConsentAllowlists,

    /// Partition NACK timeout per architecture §7.2 "configurable timeout
    /// (default 30s)". Valid range 1..=600.
    #[serde(default = "default_partition_timeout_secs")]
    pub partition_timeout_secs: u64,

    /// Story 8.9 / AC3 (Decision §D1) — per-peer consent-envelope TTL in
    /// seconds. When the sender's `prepare_outbound` synthesizes a bounded
    /// expiry for an envelope that carries none, it stamps
    /// `now + consent_ttl_secs`. An envelope that already carries an explicit
    /// `valid_until_ns` (an authoritative grant) is left untouched. Mirrors the
    /// `partition_timeout_secs` operator-tunable pattern; default 300, valid
    /// range 1..=86400.
    #[serde(default = "default_consent_ttl_secs")]
    pub consent_ttl_secs: u64,
}

fn default_profile() -> A2AProfile {
    A2AProfile::Loopback
}

fn default_partition_timeout_secs() -> u64 {
    30
}

/// Story 8.9 / AC3 (Decision §D1) — default per-peer consent TTL. 300s mirrors
/// the architecture's default consent window; operators tune per peer.
pub const DEFAULT_CONSENT_TTL_SECS: u64 = 300;

fn default_consent_ttl_secs() -> u64 {
    DEFAULT_CONSENT_TTL_SECS
}

impl A2APeerConfig {
    /// Validate the per-peer config — called at composition-root admission
    /// time per AC3. Rejects out-of-range `partition_timeout_secs` and
    /// non-`tls://` endpoints.
    pub fn validate(&self) -> Result<(), crate::error::A2AError> {
        if !(1..=600).contains(&self.partition_timeout_secs) {
            return Err(crate::error::A2AError::ConfigInvalid(format!(
                "partition_timeout_secs must be in 1..=600, got {}",
                self.partition_timeout_secs
            )));
        }
        // Story 8.9 / AC3 — consent TTL must be a bounded, operator-tunable
        // security window (Decision §D1): never 0 (no expiry window) and never
        // absurdly long.
        if !(1..=86400).contains(&self.consent_ttl_secs) {
            return Err(crate::error::A2AError::ConfigInvalid(format!(
                "consent_ttl_secs must be in 1..=86400, got {}",
                self.consent_ttl_secs
            )));
        }
        if !self.endpoint.starts_with("tls://") {
            return Err(crate::error::A2AError::ConfigInvalid(format!(
                "endpoint scheme must be 'tls://', got {}",
                self.endpoint
            )));
        }
        // Validate the host:port portion after "tls://"
        let rest = &self.endpoint[6..]; // skip "tls://"
        if rest.is_empty() {
            return Err(crate::error::A2AError::ConfigInvalid(
                "endpoint must specify a host after 'tls://'".into(),
            ));
        }
        // Basic host:port validation — accept hostname, IPv4, or IPv6
        if let Some(colon) = rest.rfind(':') {
            let port_str = &rest[colon + 1..];
            if port_str.is_empty() || port_str.parse::<u16>().is_err() {
                return Err(crate::error::A2AError::ConfigInvalid(format!(
                    "endpoint port '{port_str}' is not a valid u16 port number"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::invariants::i8::A2AIntent;

    #[test]
    fn config_round_trip_toml() {
        let toml_src = r#"
            [[peer]]
            peer_id = "host-b-prod-edge"
            endpoint = "tls://host-b.internal:7443"
            cert_fingerprint = { algo = "sha256", hex = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789" }
            profile = "cross_host"
            send_allowlist = ["diagnosis-handoff:read-only-evidence"]
            accept_allowlist = ["rca-summary"]
            partition_timeout_secs = 30
        "#;
        let cfg: A2AConfig = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.peer.len(), 1);
        assert_eq!(cfg.peer[0].peer_id.as_str(), "host-b-prod-edge");
        assert!(matches!(cfg.peer[0].profile, A2AProfile::CrossHost));
        assert_eq!(
            cfg.peer[0].allowlists.send_allowlist.len(),
            1
        );
    }

    #[test]
    fn config_defaults_loopback_profile_and_30s_timeout() {
        let toml_src = r#"
            [[peer]]
            peer_id = "loopback"
            endpoint = "tls://127.0.0.1:7443"
            cert_fingerprint = { algo = "sha256", hex = "0000000000000000000000000000000000000000000000000000000000000000" }
        "#;
        let cfg: A2AConfig = toml::from_str(toml_src).expect("parse");
        assert!(matches!(cfg.peer[0].profile, A2AProfile::Loopback));
        assert_eq!(cfg.peer[0].partition_timeout_secs, 30);
    }

    #[test]
    fn config_rejects_unknown_field() {
        let toml_src = r#"
            [[peer]]
            peer_id = "loopback"
            endpoint = "tls://127.0.0.1:7443"
            cert_fingerprint = { algo = "sha256", hex = "0000000000000000000000000000000000000000000000000000000000000000" }
            bogus_field = "should fail"
        "#;
        assert!(toml::from_str::<A2AConfig>(toml_src).is_err());
    }

    #[test]
    fn validate_rejects_out_of_range_timeout() {
        let peer = A2APeerConfig {
            peer_id: PeerId::new("p"),
            endpoint: "tls://127.0.0.1:7443".into(),
            cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
            profile: A2AProfile::Loopback,
            allowlists: ConsentAllowlists::default(),
            partition_timeout_secs: 0,
            consent_ttl_secs: DEFAULT_CONSENT_TTL_SECS,
        };
        assert!(peer.validate().is_err());
        let peer_too_big = A2APeerConfig {
            partition_timeout_secs: 601,
            ..peer
        };
        assert!(peer_too_big.validate().is_err());
    }

    #[test]
    fn validate_rejects_non_tls_scheme() {
        let peer = A2APeerConfig {
            peer_id: PeerId::new("p"),
            endpoint: "http://x".into(),
            cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
            profile: A2AProfile::Loopback,
            allowlists: ConsentAllowlists::default(),
            partition_timeout_secs: 30,
            consent_ttl_secs: DEFAULT_CONSENT_TTL_SECS,
        };
        assert!(peer.validate().is_err());
    }

    // ── Story 8.9 / AC3 — operator-configurable consent TTL ───────────────────

    #[test]
    fn config_consent_ttl_round_trip_and_default_applies() {
        // Explicit value round-trips.
        let toml_src = r#"
            [[peer]]
            peer_id = "host-b"
            endpoint = "tls://host-b.internal:7443"
            cert_fingerprint = { algo = "sha256", hex = "0000000000000000000000000000000000000000000000000000000000000000" }
            consent_ttl_secs = 120
        "#;
        let cfg: A2AConfig = toml::from_str(toml_src).expect("parse");
        assert_eq!(cfg.peer[0].consent_ttl_secs, 120);

        // Absent → default applies (additive + defaulted under deny_unknown_fields).
        let toml_default = r#"
            [[peer]]
            peer_id = "host-b"
            endpoint = "tls://host-b.internal:7443"
            cert_fingerprint = { algo = "sha256", hex = "0000000000000000000000000000000000000000000000000000000000000000" }
        "#;
        let cfg2: A2AConfig = toml::from_str(toml_default).expect("parse");
        assert_eq!(cfg2.peer[0].consent_ttl_secs, DEFAULT_CONSENT_TTL_SECS);
    }

    #[test]
    fn validate_rejects_out_of_range_consent_ttl() {
        let base = A2APeerConfig {
            peer_id: PeerId::new("p"),
            endpoint: "tls://127.0.0.1:7443".into(),
            cert_fingerprint: PeerCertFingerprint::from_cert_der(b"x"),
            profile: A2AProfile::Loopback,
            allowlists: ConsentAllowlists::default(),
            partition_timeout_secs: 30,
            consent_ttl_secs: 0,
        };
        assert!(base.validate().is_err(), "consent_ttl_secs = 0 must be rejected");
        let too_big = A2APeerConfig {
            consent_ttl_secs: 86401,
            ..base.clone()
        };
        assert!(too_big.validate().is_err(), "consent_ttl_secs = 86401 must be rejected");
        let ok = A2APeerConfig {
            consent_ttl_secs: 86400,
            ..base
        };
        assert!(ok.validate().is_ok(), "consent_ttl_secs = 86400 (boundary) must pass");
    }

    #[test]
    fn allowlist_admission_via_config() {
        let toml_src = r#"
            [[peer]]
            peer_id = "p"
            endpoint = "tls://127.0.0.1:7443"
            cert_fingerprint = { algo = "sha256", hex = "0000000000000000000000000000000000000000000000000000000000000000" }
            send_allowlist = ["intent-x"]
            accept_allowlist = ["intent-y"]
        "#;
        let cfg: A2AConfig = toml::from_str(toml_src).expect("parse");
        let p = &cfg.peer[0];
        assert!(p.allowlists.send_admits(&A2AIntent::new("intent-x")));
        assert!(p.allowlists.accept_admits(&A2AIntent::new("intent-y")));
        assert!(!p.allowlists.send_admits(&A2AIntent::new("nope")));
    }
}
