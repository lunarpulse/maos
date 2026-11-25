//! Story 23-7 — the SPIFFE X509-SVID URI SAN that names a cohort peer in its leaf.
//!
//! D-23-7-04 A: exactly one URI SAN, `spiffe://<trust_domain>/maos/peer/<peer_id>`; more than one
//! URI SAN is refused (X509-SVID §2: validators MUST reject it). D-23-7-05 A: the SAN is read with
//! `x509-cert`. D-23-7-06 A: `peer_san = "required"` scopes the listen-side pin check to the named
//! peer; `"off"` keeps the flat lookup.
//!
//! The SAN only SELECTS which pin record the handshake verifies against (ADR-063): it grants
//! nothing, and the fingerprint comparison still decides.

use maos_a2a_core::identity::PeerId;
use serde::{Deserialize, Serialize};
use x509_cert::der::oid::AssociatedOid;
use x509_cert::der::Decode;
use x509_cert::ext::pkix::name::GeneralName;
use x509_cert::ext::pkix::SubjectAltName;
use x509_cert::Certificate;

const SCHEME: &str = "spiffe://";
const PEER_PATH: &str = "/maos/peer/";

/// `[tcp] peer_san`: whether the listen side reads the client leaf's SAN.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PeerSanMode {
    /// The flat listen-side lookup (any active pin), byte for byte as before Story 23-7.
    #[default]
    Off,
    /// The listen side requires one SPIFFE URI SAN naming a pinned peer.
    Required,
}

/// Why a leaf's SAN did not name a peer (every case is a refused handshake under `required`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SanRefusal {
    /// The certificate (or its SAN extension) could not be parsed.
    Unparseable,
    /// No URI SAN.
    Missing,
    /// More than one URI SAN (or more than one SAN extension).
    Multiple,
    /// The URI is not `spiffe://<trust_domain>/maos/peer/<peer_id>` for the configured domain.
    ForeignTrustDomain,
    /// A `spiffe://` URI in the configured domain whose path is not a valid peer id.
    Malformed,
}

impl std::fmt::Display for SanRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SanRefusal::Unparseable => "certificate or SAN extension unparseable",
            SanRefusal::Missing => "no URI SAN",
            SanRefusal::Multiple => "more than one URI SAN",
            SanRefusal::ForeignTrustDomain => "URI SAN outside the configured trust domain",
            SanRefusal::Malformed => "URI SAN is not a MAOS peer identity",
        })
    }
}

/// SPIFFE path-segment characters `[a-zA-Z0-9._-]`; `.` and `..` are not valid segments.
pub fn is_peer_id(text: &str) -> bool {
    !text.is_empty()
        && text != "."
        && text != ".."
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// SPIFFE trust-domain characters: lowercase letters, digits, `.`, `-`, `_`.
pub fn is_trust_domain(text: &str) -> bool {
    !text.is_empty()
        && text.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

/// The SPIFFE ID a leaf must carry to name `peer` in `trust_domain`.
pub fn spiffe_id(trust_domain: &str, peer: &str) -> String {
    format!("{SCHEME}{trust_domain}{PEER_PATH}{peer}")
}

/// The peer a leaf's SAN names under `trust_domain`, or why it names none.
pub fn peer_from_leaf(der: &[u8], trust_domain: &str) -> Result<PeerId, SanRefusal> {
    let certificate = Certificate::from_der(der).map_err(|_| SanRefusal::Unparseable)?;
    let mut extensions = certificate
        .tbs_certificate
        .extensions
        .iter()
        .flatten()
        .filter(|extension| extension.extn_id == SubjectAltName::OID);
    let (Some(extension), None) = (extensions.next(), extensions.next()) else {
        return Err(if certificate_has_no_san(&certificate) {
            SanRefusal::Missing
        } else {
            SanRefusal::Multiple
        });
    };
    let names = SubjectAltName::from_der(extension.extn_value.as_bytes())
        .map_err(|_| SanRefusal::Unparseable)?
        .0;
    let mut uris = names.iter().filter_map(|name| match name {
        GeneralName::UniformResourceIdentifier(uri) => Some(uri.as_str()),
        _ => None,
    });
    let (Some(uri), None) = (uris.next(), uris.next()) else {
        return Err(
            if names
                .iter()
                .any(|name| matches!(name, GeneralName::UniformResourceIdentifier(_)))
            {
                SanRefusal::Multiple
            } else {
                SanRefusal::Missing
            },
        );
    };
    peer_from_uri(uri, trust_domain)
}

fn certificate_has_no_san(certificate: &Certificate) -> bool {
    certificate
        .tbs_certificate
        .extensions
        .iter()
        .flatten()
        .all(|extension| extension.extn_id != SubjectAltName::OID)
}

fn peer_from_uri(uri: &str, trust_domain: &str) -> Result<PeerId, SanRefusal> {
    let rest = uri
        .strip_prefix(SCHEME)
        .ok_or(SanRefusal::ForeignTrustDomain)?;
    let (domain, path) = rest.split_once('/').ok_or(SanRefusal::ForeignTrustDomain)?;
    if domain != trust_domain {
        return Err(SanRefusal::ForeignTrustDomain);
    }
    let peer = format!("/{path}")
        .strip_prefix(PEER_PATH)
        .map(str::to_owned)
        .ok_or(SanRefusal::Malformed)?;
    if is_peer_id(&peer) {
        Ok(PeerId::new(peer))
    } else {
        Err(SanRefusal::Malformed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_forms() {
        let ok =
            |uri: &str| peer_from_uri(uri, "corp.example").map(|peer| peer.as_str().to_owned());
        assert_eq!(
            ok("spiffe://corp.example/maos/peer/host_a"),
            Ok("host_a".into())
        );
        assert_eq!(
            ok("spiffe://corp.example/maos/peer/h-1.b_2"),
            Ok("h-1.b_2".into())
        );
        assert_eq!(
            ok("spiffe://other.example/maos/peer/host_a"),
            Err(SanRefusal::ForeignTrustDomain)
        );
        assert_eq!(
            ok("urn:maos:peer:host_a"),
            Err(SanRefusal::ForeignTrustDomain)
        );
        for malformed in [
            "spiffe://corp.example/maos/peer/",
            "spiffe://corp.example/maos/peer/..",
            "spiffe://corp.example/maos/peer/a/b",
            "spiffe://corp.example/maos/peer/a?x",
            "spiffe://corp.example/maos/peer/a#x",
            "spiffe://corp.example/other/host_a",
            "spiffe://corp.example/",
        ] {
            assert_eq!(ok(malformed), Err(SanRefusal::Malformed), "{malformed}");
        }
        assert!(
            is_trust_domain("corp.example") && !is_trust_domain("Corp") && !is_trust_domain("")
        );
        assert!(is_peer_id("host_a") && !is_peer_id("a b") && !is_peer_id(""));
    }
}
