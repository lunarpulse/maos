#![forbid(unsafe_code)]

//! CRL parser — JSON decode, schema version check, trust-anchor pin,
//! Ed25519 signature verification, entry validation.

use maos_domain::ports::crypto::CryptoProvider;
use maos_domain::revocation::{canonical_entries_bytes, RevocationError, SignedRevocationList};
/// Parse a signed CRL from raw bytes.
///
/// Steps:
/// 1. Decode JSON → `SignedRevocationList`
/// 2. `schema_version == 1` check
/// 3. `signer_pub_key == trust_anchor_pub` pin check
/// 4. `CryptoProvider::verify_signature` over canonical-serialized entries
/// 5. Validate every entry's `version_range` parses
pub fn parse_signed_crl(
    bytes: &[u8],
    trust_anchor_pub: &[u8],
    crypto: &dyn CryptoProvider,
) -> Result<SignedRevocationList, RevocationError> {
    // 1. Decode JSON
    let crl: SignedRevocationList =
        serde_json::from_slice(bytes).map_err(|e| RevocationError::Deserialize(e.to_string()))?;

    // Deserialization bypasses constructors.  Re-run every constructor
    // invariant so wire input and locally-created CRLs have one contract.
    let validated_entries = crl
        .entries
        .iter()
        .map(|entry| {
            maos_domain::revocation::RevocationEntry::new(
                entry.spirit_class.clone(),
                entry.version_range.clone(),
                entry.reason.clone(),
                entry.recommended_action,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let crl = SignedRevocationList::new(
        crl.id,
        crl.schema_version,
        crl.issued_at_ns,
        crl.origin,
        validated_entries,
        crl.signature,
        crl.signer_pub_key,
    )?;

    // 3. Trust anchor pin check
    if crl.signer_pub_key.as_slice() != trust_anchor_pub {
        return Err(RevocationError::TrustAnchorMismatch {
            observed: hex::encode(crl.signer_pub_key),
        });
    }

    let entries_bytes = canonical_entries_bytes(&crl.entries)?;
    crypto
        .verify_signature(&crl.signer_pub_key, &entries_bytes, &crl.signature)
        .map_err(|_| RevocationError::SignatureInvalid)?;

    // `RevocationEntry::new` above validates both the canonical lowercase
    // class and the version range.  No partially validated wire object may
    // reach matching/admission.

    Ok(crl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RingCryptoProvider;
    use maos_domain::revocation::{CrlId, RevocationEntry, RevocationOrigin};
    use ring::signature::{Ed25519KeyPair, KeyPair};

    fn valid_crl() -> (SignedRevocationList, [u8; 32]) {
        let seed = [7u8; 32];
        let keypair = Ed25519KeyPair::from_seed_unchecked(&seed).unwrap();
        let public: [u8; 32] = keypair.public_key().as_ref().try_into().unwrap();
        let entries =
            vec![
                RevocationEntry::new("hello-spirit", ">=0.1.0,<0.2.0", "compromised", None)
                    .unwrap(),
            ];
        let bytes = canonical_entries_bytes(&entries).unwrap();
        let signature: [u8; 64] = RingCryptoProvider
            .sign_capability_token(&seed, &bytes)
            .unwrap()
            .try_into()
            .unwrap();
        (
            SignedRevocationList::new(
                CrlId::from_entries(&entries).unwrap(),
                1,
                0,
                RevocationOrigin::Operator,
                entries,
                signature,
                public,
            )
            .unwrap(),
            public,
        )
    }

    #[test]
    fn parse_accepts_real_ed25519_signature_over_canonical_entries() {
        let (crl, public) = valid_crl();
        let bytes = serde_json::to_vec(&crl).unwrap();
        let parsed = parse_signed_crl(&bytes, &public, &RingCryptoProvider).unwrap();
        assert_eq!(parsed.id, CrlId::from_entries(&parsed.entries).unwrap());
    }

    #[test]
    fn parse_rejects_wrong_schema_version() {
        let (mut crl, public) = valid_crl();
        crl.schema_version = 2;
        let bytes = serde_json::to_vec(&crl).unwrap();
        let err = parse_signed_crl(&bytes, &public, &RingCryptoProvider).unwrap_err();
        assert!(matches!(
            err,
            RevocationError::UnsupportedSchemaVersion { actual: 2 }
        ));
    }

    #[test]
    fn parse_rejects_trust_anchor_mismatch() {
        let (crl, _) = valid_crl();
        let bytes = serde_json::to_vec(&crl).unwrap();
        let err = parse_signed_crl(&bytes, &[99u8; 32], &RingCryptoProvider).unwrap_err();
        assert!(matches!(err, RevocationError::TrustAnchorMismatch { .. }));
    }

    #[test]
    fn parse_rejects_mutated_real_signature() {
        let (mut crl, public) = valid_crl();
        crl.signature[0] ^= 0xFF;
        let bytes = serde_json::to_vec(&crl).unwrap();
        assert!(matches!(
            parse_signed_crl(&bytes, &public, &RingCryptoProvider),
            Err(RevocationError::SignatureInvalid)
        ));
    }

    #[test]
    fn parse_rejects_noncanonical_wire_class_before_matching() {
        let (mut crl, public) = valid_crl();
        crl.entries[0].spirit_class = "Hello-Spirit".into();
        let bytes = serde_json::to_vec(&crl).unwrap();
        assert!(matches!(
            parse_signed_crl(&bytes, &public, &RingCryptoProvider),
            Err(RevocationError::MalformedVersionRange { .. })
        ));
    }
}
