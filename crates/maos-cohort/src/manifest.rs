#![forbid(unsafe_code)]
//! Cohort manifest schema v1 — the signed TOML roster that drives a full-
//! pairwise A2A mesh (Story 12.1 / Task 1).
//!
//! A cohort manifest declares the members (each pinned to its §7.2 mTLS
//! cert fingerprint), their roles, the per-(peer,role) consent matrix as
//! **split send/accept tables** (no transposition ambiguity), the genesis
//! cohort authority `{ keys, threshold }`, the two schema-mandatory reserved
//! always-allowlisted intents, the signed `t_stale_secs` staleness ceiling,
//! and a strictly-monotonic integer `version`. The whole body is Ed25519-
//! signed by the genesis authority.
//!
//! # Trust model (R1 / AC3 / RR3–RR6)
//!
//! - **Genesis trust is operator-pinned out of band** — each member holds the
//!   authority pubkey in [`crate::pin::PinnedAuthorityKeys`], provisioned
//!   before the first manifest is seen (same posture as the §7.2 cert pin).
//!   TOFU-on-first-manifest is NOT used for the authority key.
//! - The manifest's **declared** authority MUST be a subset of the pinned set
//!   or the manifest is refused ([`CohortError::ECohortAuthorityUnpinned`]).
//! - The signature is verified against the **pinned** key set — NEVER a key
//!   carried in the manifest body, NEVER re-derived from the manifest's own
//!   declaration (Ed25519 has no key recovery; the verifier holds the key).
//! - `threshold > 1` is rejected at v1 ([`CohortError::EUnsupportedAuthorityScheme`]).
//! - `t_stale_secs` is clamped to code-owned `[T_STALE_MIN, T_STALE_MAX]`
//!   bounds (the signed field tunes within bounds; it never defines them).
//!
//! # Canonical signing bytes (pinned — build/verify MUST be symmetric)
//!
//! Mirrors the `maos-loom-lite` `leaf.rs` idiom: a fixed domain separator,
//! fixed-width big-endian scalars, and a **4-byte big-endian length prefix on
//! every variable-length field**. The length prefix is load-bearing — it makes
//! a boundary shift (e.g. `cohort_id="ab",host_id="c"` vs `"a","bc"`)
//! canonically distinct (see `boundary_shift_no_collision`).

#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier};
use maos_a2a_core::{A2APeerConfig, A2AProfile, ConsentAllowlists, PeerCertFingerprint, PeerId};
use maos_domain::invariants::i8::A2AIntent;
use serde::{Deserialize, Serialize};

use crate::error::CohortError;
use crate::pin::PinnedAuthorityKeys;

/// Frozen manifest schema version. Bumping is a wire-format change that MUST
/// be gated by an ADR.
pub const SCHEMA_VERSION: u64 = 1;

/// Domain separator bound into every cohort-manifest signature. Pinned — never
/// change without an ADR, because doing so re-keys every in-flight manifest.
pub const SIG_DOMAIN: &[u8] = b"maos.cohort-manifest.v1";

/// The two schema-mandatory reserved always-allowlisted intent subtypes
/// (colon-kebab — the dotted arch-text form FAILS `A2AIntent::is_canonical`
/// and can never match an allowlist; CATCH-0 #1).
pub const RESERVED_INTENT_REISSUE: &str = "cohort:manifest-reissue";
pub const RESERVED_INTENT_HALT_RECEIPT: &str = "cohort:halt-receipt";

/// Code-owned staleness bounds (RR6). The signed `t_stale_secs` field tunes
/// WITHIN these bounds; it never defines them — else the authority signs away
/// its own fail-closed staleness (e.g. a 1yr ceiling lets a revoked member
/// live a year).
///
/// - `T_STALE_MIN` = 30s — at least the §7.2 partition window
///   (`default_partition_timeout_secs` = 30); a stale ceiling below the
///   partition timeout is self-defeating.
/// - `T_STALE_MAX` = 3600s — a hard 1-hour ceiling bounding revoked-member
///   exposure.
/// - `T_STALE_DEFAULT` = 120s — the documented default when the field is
///   omitted (§15.2 architecture decision).
pub const T_STALE_MIN: u64 = 30;
pub const T_STALE_MAX: u64 = 3600;
pub const T_STALE_DEFAULT: u64 = 120;

/// §7.2 default per-peer partition NACK timeout (seconds) projected by
/// [`CohortManifest::peer_configs_for`] (Task 2). The v1 manifest carries no
/// per-peer override, so the architecture default ("configurable timeout
/// (default 30s)") applies — mirroring `maos-a2a-core`'s operator-config
/// default.
pub const PEER_PARTITION_TIMEOUT_SECS: u64 = 30;

/// §7.2 default per-peer consent-envelope TTL (seconds) projected by
/// [`CohortManifest::peer_configs_for`] (Task 2). The v1 manifest carries no
/// per-peer override; this mirrors `maos-a2a-core`'s `DEFAULT_CONSENT_TTL_SECS`
/// (Decision §D1, the documented 300s default consent window).
pub const PEER_CONSENT_TTL_SECS: u64 = 300;

fn default_t_stale_secs() -> u64 {
    T_STALE_DEFAULT
}

// ─── Schema types ──────────────────────────────────────────────────────────

/// Genesis cohort authority declaration.
///
/// `keys` is the declared authority key set (hex-encoded 32-byte Ed25519
/// verifying keys). At parse it MUST be a subset of the operator-pinned set
/// (R1/RR5) or the manifest is refused. `threshold` is kept for forward-compat
/// but v1 rejects `threshold > 1` (R2).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortAuthority {
    pub threshold: u64,
    pub keys: Vec<String>,
}

/// A cohort member — pinned to its §7.2 mTLS cert fingerprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortMember {
    pub host_id: String,
    /// `sha256:<hex64>` wire form, validated by `PeerCertFingerprint::parse`.
    pub fingerprint: String,
    pub roles: Vec<String>,
}

/// A single `(peer, role, intent)` consent grant. Appears in either the
/// `send` or the `accept` table ([`ConsentMatrix`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentTuple {
    /// MUST be a declared member `host_id` (RR-min referential integrity).
    pub peer: String,
    /// MUST be a role some member declares (RR-min referential integrity).
    pub role: String,
    /// A canonical A2A intent (`A2AIntent::is_canonical`).
    pub intent: String,
}

/// Per-(peer,role) consent matrix as **split send/accept tables** — separate
/// directions remove the transposition ambiguity of a single combined table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentMatrix {
    #[serde(default)]
    pub send: Vec<ConsentTuple>,
    #[serde(default)]
    pub accept: Vec<ConsentTuple>,
}

/// Ed25519 signature block. Carries ONLY the 64-byte signature — NEVER a key.
/// The signature is verified against the operator-pinned key set (AC3), so no
/// manifest-carried key is trusted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestSignature {
    /// 64-byte Ed25519 signature, hex-encoded (128 chars).
    pub sig: String,
}

/// A signed cohort manifest (schema v1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CohortManifest {
    pub schema_version: u64,
    pub cohort_id: String,
    /// Strictly-monotonic integer; parse rejects `version < 1`.
    pub version: u64,
    pub authority: CohortAuthority,
    pub members: Vec<CohortMember>,
    #[serde(default)]
    pub consent: ConsentMatrix,
    pub reserved_intents: Vec<String>,
    #[serde(default = "default_t_stale_secs")]
    pub t_stale_secs: u64,
    pub signature: ManifestSignature,
}

impl CohortManifest {
    /// Serialize the manifest BODY (everything except the signature block)
    /// into the pinned canonical byte form. Build/verify MUST stay symmetric.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(SIG_DOMAIN);
        buf.extend_from_slice(&self.schema_version.to_be_bytes());
        write_lp_bytes(&mut buf, self.cohort_id.as_bytes());
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.t_stale_secs.to_be_bytes());
        buf.extend_from_slice(&self.authority.threshold.to_be_bytes());

        // Authority keys: set semantics — sort the lowercased hex so key
        // declaration order never perturbs the signature.
        let mut keys_sorted: Vec<String> = self
            .authority
            .keys
            .iter()
            .map(|k| k.to_lowercase())
            .collect();
        keys_sorted.sort_unstable();
        buf.extend_from_slice(&(keys_sorted.len() as u32).to_be_bytes());
        for k in &keys_sorted {
            write_lp_bytes(&mut buf, k.as_bytes());
        }

        buf.extend_from_slice(&(self.members.len() as u32).to_be_bytes());
        for m in &self.members {
            write_lp_bytes(&mut buf, m.host_id.as_bytes());
            write_lp_bytes(&mut buf, m.fingerprint.as_bytes());
            buf.extend_from_slice(&(m.roles.len() as u32).to_be_bytes());
            for r in &m.roles {
                write_lp_bytes(&mut buf, r.as_bytes());
            }
        }

        canonicalize_consent_table(&mut buf, &self.consent.send);
        canonicalize_consent_table(&mut buf, &self.consent.accept);

        buf.extend_from_slice(&(self.reserved_intents.len() as u32).to_be_bytes());
        for ri in &self.reserved_intents {
            write_lp_bytes(&mut buf, ri.as_bytes());
        }

        buf
    }

    /// SHA-256 of the canonical body — convenience mirroring `leaf.rs`'s
    /// `canonical_hash`, used by the boundary-shift collision test.
    pub fn canonical_hash(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.to_canonical_bytes());
        hasher.finalize().into()
    }

    /// Parse + strict schema-validate a signed TOML manifest against the
    /// operator-pinned genesis authority key set.
    ///
    /// Performs every Task 1 structural invariant (schema version, version ≥
    /// 1, no duplicate host_id, §7.2 fingerprint form, non-empty authority,
    /// threshold ≤ 1, declared-authority ⊆ pinned, t_stale bounds, reserved
    /// intents present + canonical, consent referential integrity). The
    /// cryptographic signature is verified separately via
    /// [`CohortManifest::verify_signature`] against the pinned keys (AC3).
    pub fn parse_and_validate(
        src: &str,
        pinned: &PinnedAuthorityKeys,
    ) -> Result<Self, CohortError> {
        let mut manifest: CohortManifest =
            toml::from_str(src).map_err(|e| CohortError::ParseError(e.to_string()))?;

        if manifest.schema_version != SCHEMA_VERSION {
            return Err(CohortError::EUnsupportedSchemaVersion {
                got: manifest.schema_version,
                expected: SCHEMA_VERSION,
            });
        }
        if manifest.version < 1 {
            return Err(CohortError::EVersionNotPositive {
                version: manifest.version,
            });
        }
        if manifest.members.is_empty() {
            return Err(CohortError::EEmptyMembers);
        }

        // No duplicate host_id.
        let mut seen_hosts: Vec<&str> = manifest
            .members
            .iter()
            .map(|m| m.host_id.as_str())
            .collect();
        seen_hosts.sort_unstable();
        for w in seen_hosts.windows(2) {
            if w[0] == w[1] {
                return Err(CohortError::EDuplicateHostId {
                    host_id: w[0].to_string(),
                });
            }
        }

        // §7.2 fingerprint form.
        for m in &manifest.members {
            if PeerCertFingerprint::parse(&m.fingerprint).is_none() {
                return Err(CohortError::EInvalidFingerprint {
                    host_id: m.host_id.clone(),
                    fingerprint: m.fingerprint.clone(),
                });
            }
        }

        // Authority: non-empty, threshold <= 1, declared keys are valid Ed25519
        // pubkeys (normalized to lowercase), and every declared key is pinned.
        if manifest.authority.keys.is_empty() {
            return Err(CohortError::EEmptyAuthority);
        }
        if manifest.authority.threshold > 1 {
            return Err(CohortError::EUnsupportedAuthorityScheme {
                threshold: manifest.authority.threshold,
            });
        }
        // Normalize declared key hex to lowercase (canonical determinism).
        for k in manifest.authority.keys.iter_mut() {
            *k = k.to_lowercase();
            crate::pin::parse_verifying_key(k)?;
        }

        let pinned_bytes: Vec<[u8; 32]> = pinned.key_bytes();
        let unpinned: Vec<String> = manifest
            .authority
            .keys
            .iter()
            .filter(|declared| {
                let dk = hex::decode(declared).unwrap_or_default();
                !pinned_bytes.iter().any(|p| p.as_slice() == dk.as_slice())
            })
            .cloned()
            .collect();
        if !unpinned.is_empty() {
            return Err(CohortError::ECohortAuthorityUnpinned {
                unpinned_count: unpinned.len(),
                unpinned,
            });
        }

        // t_stale bounds (RR6).
        if manifest.t_stale_secs < T_STALE_MIN || manifest.t_stale_secs > T_STALE_MAX {
            return Err(CohortError::ECohortStaleBoundViolation {
                value: manifest.t_stale_secs,
                min: T_STALE_MIN,
                max: T_STALE_MAX,
            });
        }

        // Reserved intents: every entry canonical + both mandatory names present.
        for ri in &manifest.reserved_intents {
            if !A2AIntent::new(ri).is_canonical() {
                return Err(CohortError::EIntentNotCanonical { intent: ri.clone() });
            }
        }
        for required in [RESERVED_INTENT_REISSUE, RESERVED_INTENT_HALT_RECEIPT] {
            if !manifest.reserved_intents.iter().any(|ri| ri == required) {
                return Err(CohortError::EMissingReservedIntent {
                    intent: required.to_string(),
                });
            }
        }

        // Every consent tuple binds the named peer to one of that same peer's
        // declared roles. A cohort-wide role union would let one member borrow
        // another member's authority.
        validate_consent_table("send", &manifest.consent.send, &manifest.members)?;
        validate_consent_table("accept", &manifest.consent.accept, &manifest.members)?;

        Ok(manifest)
    }

    /// Cryptographically verify the manifest signature against the
    /// operator-pinned genesis authority key set (AC3).
    ///
    /// Iterates each pinned key; accepts iff the signature verifies under at
    /// least one. The signature is verified against the **pinned** keys —
    /// NEVER a key carried in the manifest body, NEVER re-derived from the
    /// manifest's own declaration.
    pub fn verify_signature(&self, pinned: &PinnedAuthorityKeys) -> Result<(), CohortError> {
        let sig_bytes = hex::decode(&self.signature.sig)
            .map_err(|e| CohortError::EInvalidSignature(format!("bad hex ({e})")))?;
        let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
            CohortError::EInvalidSignature(format!(
                "expected 64 bytes (128 hex chars), got {} bytes",
                sig_bytes.len()
            ))
        })?;
        let signature = Signature::from_bytes(&sig_arr);

        let payload = self.to_canonical_bytes();
        for key in pinned.iter() {
            if key.verify(&payload, &signature).is_ok() {
                return Ok(());
            }
        }
        Err(CohortError::ESignatureVerificationFailed)
    }

    /// Sign this manifest's body in place and return it with the signature
    /// attached. The current `signature` field is ignored (the body is what is
    /// signed). Used by the genesis authority to produce a signed manifest.
    pub fn signed_with(mut self, signing_key: &SigningKey) -> Self {
        let payload = self.to_canonical_bytes();
        let sig = signing_key.sign(&payload);
        self.signature = ManifestSignature {
            sig: hex::encode(sig.to_bytes()),
        };
        self
    }

    /// Derive the N−1 peer-config edges for `self_host` from this signed
    /// manifest (Task 2 — full-pairwise A2A mesh generation).
    ///
    /// Each edge carries the member's declared §7.2 cert fingerprint (the trust
    /// pin), the `CrossHost` profile, a `tls://<host_id>:0` placeholder
    /// endpoint (the manifest carries no network address — the caller
    /// reconciles this with the real bound `SocketAddr` at transport bind), and
    /// the ADR-012 send/accept allowlists projected from the manifest's global
    /// consent matrix by counterparty.
    ///
    /// Determinism: edges are emitted in member declaration order; allowlist
    /// intents in declaration order. `self_host` MUST name a declared member,
    /// else [`CohortError::EHostNotMember`].
    pub fn peer_configs_for(&self, self_host: &str) -> Result<Vec<A2APeerConfig>, CohortError> {
        // The mesh is full-pairwise over DECLARED members only — `self_host`
        // must name one, else there is no position from which to project edges.
        if !self.members.iter().any(|m| m.host_id == self_host) {
            return Err(CohortError::EHostNotMember {
                host_id: self_host.to_string(),
            });
        }

        let mut edges = Vec::with_capacity(self.members.len().saturating_sub(1));
        for member in &self.members {
            // Exclude self — a host never dials itself in a full-pairwise mesh.
            if member.host_id == self_host {
                continue;
            }

            // The §7.2 cert-fingerprint trust pin, straight from the manifest.
            // parse_and_validate already asserted the `sha256:<hex64>` form; the
            // `.ok_or` here is a fail-closed guard for any directly-constructed
            // (un-validated) manifest handed to the projection.
            let cert_fingerprint =
                PeerCertFingerprint::parse(&member.fingerprint).ok_or_else(|| {
                    CohortError::EInvalidFingerprint {
                        host_id: member.host_id.clone(),
                        fingerprint: member.fingerprint.clone(),
                    }
                })?;

            // Project the global consent matrix by counterparty: the manifest's
            // `peer` field names the OTHER host, so this host's send/accept
            // toward `member` = the tuples that name `member`. Declaration order
            // is preserved for determinism (the matrix is already validated).
            let send_allowlist = self
                .consent
                .send
                .iter()
                .filter(|t| t.peer == member.host_id)
                .map(|t| A2AIntent::new(&t.intent))
                .collect();
            let accept_allowlist = self
                .consent
                .accept
                .iter()
                .filter(|t| t.peer == member.host_id)
                .map(|t| A2AIntent::new(&t.intent))
                .collect();

            // Endpoint placeholder: the manifest carries NO network address
            // (trust is the fingerprint pin, not a dial target). `tls://<host>:0`
            // is a deterministic, validate-clean sentinel — port 0 marks the
            // address as not-yet-materialized; the caller reconciles it with the
            // real bound `SocketAddr` (read back from the listener) before dial.
            edges.push(A2APeerConfig {
                peer_id: PeerId::new(&member.host_id),
                endpoint: format!("tls://{}:0", member.host_id),
                cert_fingerprint,
                profile: A2AProfile::CrossHost,
                allowlists: ConsentAllowlists {
                    send_allowlist,
                    accept_allowlist,
                },
                partition_timeout_secs: PEER_PARTITION_TIMEOUT_SECS,
                consent_ttl_secs: PEER_CONSENT_TTL_SECS,
            });
        }
        Ok(edges)
    }
}

/// Append a 4-byte big-endian length prefix followed by the bytes (the
/// `leaf.rs` `write_lp_bytes` idiom — load-bearing against boundary shifts).
fn write_lp_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// Canonicalize a consent table: u32-BE count, then each tuple's
/// `(peer, role, intent)` as length-prefixed strings.
fn canonicalize_consent_table(buf: &mut Vec<u8>, table: &[ConsentTuple]) {
    buf.extend_from_slice(&(table.len() as u32).to_be_bytes());
    for t in table {
        write_lp_bytes(buf, t.peer.as_bytes());
        write_lp_bytes(buf, t.role.as_bytes());
        write_lp_bytes(buf, t.intent.as_bytes());
    }
}

/// Referential-integrity + canonicality validation for one consent table:
/// each peer must be a member, its role must be declared by that same member,
/// and its intent must be canonical.
fn validate_consent_table(
    direction: &str,
    table: &[ConsentTuple],
    members: &[CohortMember],
) -> Result<(), CohortError> {
    for tuple in table {
        let Some(member) = members.iter().find(|member| member.host_id == tuple.peer) else {
            return Err(CohortError::EConsentPeerNotMember {
                direction: direction.to_string(),
                peer: tuple.peer.clone(),
            });
        };
        if !member.roles.iter().any(|role| role == &tuple.role) {
            return Err(CohortError::EConsentRoleUndeclared {
                direction: direction.to_string(),
                role: tuple.role.clone(),
            });
        }
        if !A2AIntent::new(&tuple.intent).is_canonical() {
            return Err(CohortError::EIntentNotCanonical {
                intent: tuple.intent.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Deterministic Ed25519 keys for fixtures ────────────────────────────

    fn signing_key(seed: u8) -> SigningKey {
        let mut s = [0u8; 32];
        s[0] = seed;
        s[31] = 1;
        SigningKey::from_bytes(&s)
    }

    fn pubkey_hex(seed: u8) -> String {
        hex::encode(signing_key(seed).verifying_key().to_bytes())
    }

    const FP_A: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const FP_B: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const FP_C: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    /// A minimal valid N=3 cohort manifest body (signature filled by signing).
    fn sample_manifest_body() -> CohortManifest {
        CohortManifest {
            schema_version: SCHEMA_VERSION,
            cohort_id: "marcus-team-nexus".to_string(),
            version: 1,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![pubkey_hex(1)],
            },
            members: vec![
                CohortMember {
                    host_id: "host-a".to_string(),
                    fingerprint: FP_A.to_string(),
                    roles: vec!["coordinator".to_string()],
                },
                CohortMember {
                    host_id: "host-b".to_string(),
                    fingerprint: FP_B.to_string(),
                    roles: vec!["worker".to_string()],
                },
                CohortMember {
                    host_id: "host-c".to_string(),
                    fingerprint: FP_C.to_string(),
                    roles: vec!["worker".to_string(), "reviewer".to_string()],
                },
            ],
            consent: ConsentMatrix {
                send: vec![ConsentTuple {
                    peer: "host-b".to_string(),
                    role: "worker".to_string(),
                    intent: "diagnosis-handoff:read-only-evidence".to_string(),
                }],
                accept: vec![ConsentTuple {
                    peer: "host-a".to_string(),
                    role: "coordinator".to_string(),
                    intent: "code-mutation-directive".to_string(),
                }],
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.to_string(),
                RESERVED_INTENT_HALT_RECEIPT.to_string(),
            ],
            t_stale_secs: T_STALE_DEFAULT,
            signature: ManifestSignature { sig: String::new() },
        }
    }

    fn signed_sample() -> CohortManifest {
        sample_manifest_body().signed_with(&signing_key(1))
    }

    fn pinned_authority() -> PinnedAuthorityKeys {
        PinnedAuthorityKeys::from_hex(&[pubkey_hex(1)]).unwrap()
    }

    // ── Canonical bytes ────────────────────────────────────────────────────

    #[test]
    fn canonical_bytes_deterministic() {
        let a = signed_sample().to_canonical_bytes();
        let b = signed_sample().to_canonical_bytes();
        assert_eq!(a, b, "identical manifests must canonicalize identically");
    }

    #[test]
    fn canonical_bytes_excludes_signature() {
        // The signature block MUST NOT be part of the signed payload — the
        // body canonicalization is identical regardless of the signature value.
        let mut m = signed_sample();
        let body_a = m.to_canonical_bytes();
        m.signature = ManifestSignature {
            sig: "00".repeat(64),
        };
        let body_b = m.to_canonical_bytes();
        assert_eq!(
            body_a, body_b,
            "signature field must be excluded from canonical bytes"
        );
    }

    #[test]
    fn different_field_different_hash() {
        let base = signed_sample();

        let mut cohort = base.clone();
        cohort.cohort_id = "other-cohort".to_string();
        assert_ne!(base.canonical_hash(), cohort.canonical_hash());

        let mut ver = base.clone();
        ver.version += 1;
        assert_ne!(base.canonical_hash(), ver.canonical_hash());

        let mut stale = base.clone();
        stale.t_stale_secs = 200;
        assert_ne!(base.canonical_hash(), stale.canonical_hash());

        let mut thr = base.clone();
        thr.authority.threshold = 0; // still valid field value, different bytes
        assert_ne!(base.canonical_hash(), thr.canonical_hash());

        let mut host = base.clone();
        host.members[0].host_id = "host-z".to_string();
        assert_ne!(base.canonical_hash(), host.canonical_hash());

        let mut role = base.clone();
        role.members[2].roles.push("auditor".to_string());
        assert_ne!(base.canonical_hash(), role.canonical_hash());

        let mut key = base.clone();
        key.authority.keys = vec![pubkey_hex(9)];
        assert_ne!(base.canonical_hash(), key.canonical_hash());

        let mut ri = base.clone();
        ri.reserved_intents.push("cohort:extra".to_string());
        assert_ne!(base.canonical_hash(), ri.canonical_hash());

        let mut send = base.clone();
        send.consent.send[0].intent = "diagnosis-handoff:write".to_string();
        assert_ne!(base.canonical_hash(), send.canonical_hash());
    }

    #[test]
    fn boundary_shift_no_collision() {
        // The length prefix is load-bearing: without it, concatenating
        // cohort_id + host_id could let "ab"|"c" collide with "a"|"bc".
        let mut a = signed_sample();
        a.cohort_id = "ab".to_string();
        a.members[0].host_id = "c".to_string();
        let mut b = signed_sample();
        b.cohort_id = "a".to_string();
        b.members[0].host_id = "bc".to_string();
        assert_ne!(
            a.canonical_hash(),
            b.canonical_hash(),
            "boundary shift across cohort_id/host_id MUST NOT collide (length prefix is load-bearing)"
        );
        // Also assert the raw bytes differ — the hash divergence is caused by
        // the length-prefixed bytes differing, not by hash accident.
        assert_ne!(a.to_canonical_bytes(), b.to_canonical_bytes());
    }

    // ── Parse + validate happy path + round-trip ───────────────────────────

    #[test]
    fn round_trip_sign_parse_validate_verify_ok() {
        let signed = signed_sample();
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let parsed = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap();
        parsed.verify_signature(&pinned).unwrap();
    }

    #[test]
    fn t_stale_defaults_to_120_when_omitted() {
        // Serialize a signed manifest, drop the explicit `t_stale_secs` line,
        // and confirm the serde default (120) fires on re-parse.
        let signed = signed_sample();
        let toml_str: String = toml::to_string(&signed)
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with("t_stale_secs"))
            .collect::<Vec<_>>()
            .join("\n");
        let pinned = pinned_authority();
        let parsed = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap();
        assert_eq!(parsed.t_stale_secs, T_STALE_DEFAULT);
    }

    // ── Rejection legs (each asserts the SPECIFIC error discriminant) ───────

    #[test]
    fn reject_bad_schema_version() {
        // schema_version is a required field; mutate via re-serialization.
        let mut m = signed_sample();
        m.schema_version = 2;
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EUnsupportedSchemaVersion {
                got: 2,
                expected: 1
            }
        ));
    }

    #[test]
    fn reject_version_le_zero() {
        let mut m = signed_sample();
        m.version = 0;
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EVersionNotPositive { version: 0 }
        ));
    }

    #[test]
    fn reject_duplicate_host_id() {
        let mut m = signed_sample();
        m.members[2].host_id = "host-a".to_string(); // dup of member 0
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(
            matches!(err, CohortError::EDuplicateHostId { ref host_id } if host_id == "host-a")
        );
    }

    #[test]
    fn reject_bad_fingerprint_form() {
        let mut m = signed_sample();
        m.members[1].fingerprint = "not-a-fingerprint".to_string();
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EInvalidFingerprint { ref host_id, .. } if host_id == "host-b"
        ));
    }

    #[test]
    fn reject_empty_authority() {
        let mut m = signed_sample();
        m.authority.keys = vec![];
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(err, CohortError::EEmptyAuthority));
    }

    #[test]
    fn reject_threshold_gt_one() {
        // R2: threshold > 1 → EUnsupportedAuthorityScheme.
        let mut m = signed_sample();
        m.authority.threshold = 3;
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EUnsupportedAuthorityScheme { threshold: 3 }
        ));
    }

    #[test]
    fn reject_authority_not_pinned() {
        // R1/RR5: declared authority key is absent from the pinned set.
        let mut m = signed_sample();
        m.authority.keys = vec![pubkey_hex(99)]; // not pinned
                                                 // Re-sign so the manifest is otherwise well-formed (signature will be
                                                 // irrelevant — parse rejects on the unpinned authority BEFORE verify).
        let signed = m.signed_with(&signing_key(99));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::ECohortAuthorityUnpinned {
                unpinned_count: 1,
                ..
            }
        ));
    }

    #[test]
    fn accept_authority_subset_of_pinned_rotation_overlap() {
        // RR5: pinned = {current, next}; declared = {current} → OK (rotation).
        let mut m = signed_sample();
        m.authority.keys = vec![pubkey_hex(1)]; // current
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = PinnedAuthorityKeys::from_hex(&[pubkey_hex(1), pubkey_hex(2)]).unwrap(); // {current, next}
        let parsed = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap();
        parsed.verify_signature(&pinned).unwrap(); // signed by current → verifies
    }

    #[test]
    fn reject_t_stale_below_min() {
        let mut m = signed_sample();
        m.t_stale_secs = T_STALE_MIN - 1;
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::ECohortStaleBoundViolation { value, min, max }
              if value == T_STALE_MIN - 1 && min == T_STALE_MIN && max == T_STALE_MAX
        ));
    }

    #[test]
    fn reject_t_stale_above_max() {
        let mut m = signed_sample();
        m.t_stale_secs = T_STALE_MAX + 1; // e.g. a signer attempting 1yr
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::ECohortStaleBoundViolation { .. }
        ));
    }

    #[test]
    fn reject_missing_reserved_intent() {
        let mut m = signed_sample();
        m.reserved_intents = vec![RESERVED_INTENT_REISSUE.to_string()]; // missing halt-receipt
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EMissingReservedIntent { ref intent } if intent == RESERVED_INTENT_HALT_RECEIPT
        ));
    }

    #[test]
    fn reject_non_canonical_reserved_intent() {
        let mut m = signed_sample();
        // Dotted form — fails A2AIntent::is_canonical (CATCH-0 #1).
        m.reserved_intents = vec![
            "cohort.manifest.reissue".to_string(),
            RESERVED_INTENT_HALT_RECEIPT.to_string(),
        ];
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(err, CohortError::EIntentNotCanonical { .. }));
    }

    #[test]
    fn reject_consent_peer_not_member() {
        let mut m = signed_sample();
        m.consent.send[0].peer = "host-ghost".to_string(); // not a member
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EConsentPeerNotMember { ref direction, ref peer }
              if direction == "send" && peer == "host-ghost"
        ));
    }

    #[test]
    fn reject_consent_role_undeclared() {
        let mut m = signed_sample();
        m.consent.accept[0].role = "overlord".to_string(); // no member declares it
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(
            err,
            CohortError::EConsentRoleUndeclared { ref direction, ref role }
              if direction == "accept" && role == "overlord"
        ));
    }

    #[test]
    fn reject_consent_role_declared_only_by_different_peer() {
        let mut manifest = signed_sample();
        manifest.consent.send[0] = ConsentTuple {
            peer: "host-b".to_string(),
            role: "reviewer".to_string(),
            intent: "diagnosis-handoff:read-only-evidence".to_string(),
        };
        let signed = manifest.signed_with(&signing_key(1));
        let manifest_toml = toml::to_string(&signed).unwrap();
        let error =
            CohortManifest::parse_and_validate(&manifest_toml, &pinned_authority()).unwrap_err();
        assert!(matches!(
            error,
            CohortError::EConsentRoleUndeclared {
                ref direction,
                ref role,
            } if direction == "send" && role == "reviewer"
        ));
    }

    #[test]
    fn reject_consent_intent_not_canonical() {
        let mut m = signed_sample();
        m.consent.send[0].intent = "Bad Intent".to_string(); // uppercase + space
        let signed = m.signed_with(&signing_key(1));
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        assert!(matches!(err, CohortError::EIntentNotCanonical { .. }));
    }

    #[test]
    fn reject_unknown_field() {
        let signed = signed_sample();
        let base = toml::to_string(&signed).unwrap();
        let with_typo = format!("{base}\nbogus_field = 42\n");
        let pinned = pinned_authority();
        let err = CohortManifest::parse_and_validate(&with_typo, &pinned).unwrap_err();
        assert!(matches!(err, CohortError::ParseError(_)));
    }

    // ── Signature verification (AC3 — verify against pinned, never body) ────

    #[test]
    fn forged_signature_fails_verify() {
        // Sign with the impostor (key 2); the declared authority is key 1
        // and only key 1 is pinned. Parse accepts (declared==pinned key 1) but
        // the signature does not verify under any pinned key.
        let m = sample_manifest_body().signed_with(&signing_key(2));
        let toml_str = toml::to_string(&m).unwrap();
        let pinned = pinned_authority(); // pins key 1 only
        let parsed = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap();
        let err = parsed.verify_signature(&pinned).unwrap_err();
        assert!(matches!(err, CohortError::ESignatureVerificationFailed));
    }

    #[test]
    fn bad_signature_hex_rejected() {
        let mut m = signed_sample();
        m.signature = ManifestSignature {
            sig: "nothex".to_string(),
        };
        let pinned = pinned_authority();
        let err = m.verify_signature(&pinned).unwrap_err();
        assert!(matches!(err, CohortError::EInvalidSignature(_)));
    }

    #[test]
    fn bad_signature_length_rejected() {
        let mut m = signed_sample();
        m.signature = ManifestSignature {
            sig: "ff".to_string(),
        }; // 1 byte, not 64
        let pinned = pinned_authority();
        let err = m.verify_signature(&pinned).unwrap_err();
        assert!(matches!(err, CohortError::EInvalidSignature(_)));
    }

    #[test]
    fn tampered_body_fails_verify() {
        // Tamper with the body AFTER signing → canonical payload changes →
        // signature no longer verifies (the crypto integrity reflex).
        let mut m = signed_sample();
        m.cohort_id = "tampered-cohort".to_string();
        let pinned = pinned_authority();
        let err = m.verify_signature(&pinned).unwrap_err();
        assert!(matches!(err, CohortError::ESignatureVerificationFailed));
    }

    #[test]
    fn verify_never_uses_manifest_body_key() {
        // AC3: even if the manifest DECLARES an impostor key AND is signed by
        // that impostor, parse refuses it (declared ∉ pinned) — the body key
        // is never a trust anchor. We assert parse rejects BEFORE verify runs.
        let mut body = sample_manifest_body();
        body.authority.keys = vec![pubkey_hex(2)]; // declare impostor
        let signed = body.signed_with(&signing_key(2)); // sign by impostor
        let toml_str = toml::to_string(&signed).unwrap();
        let pinned = pinned_authority(); // pins key 1 only
        let err = CohortManifest::parse_and_validate(&toml_str, &pinned).unwrap_err();
        // Declared impostor key is not pinned → ECohortAuthorityUnpinned,
        // NOT a silent accept-and-verify-with-the-body-key.
        assert!(matches!(err, CohortError::ECohortAuthorityUnpinned { .. }));
    }

    // ── Task 2: peer_configs_for — signed manifest → N−1 peer edges ─────────

    /// A distinct, valid §7.2 fingerprint (`sha256:<hex64>`) for member index
    /// `i` — 64 zero-padded lowercase hex chars, distinct per i. Distinctness
    /// is the load-bearing property for the "declared fingerprint" reflex.
    fn fp_for(i: usize) -> String {
        format!("sha256:{i:064x}")
    }

    /// An N-member manifest with DISTINCT fingerprints and a full-pairwise
    /// `readonly` consent matrix: every member is a counterparty peer in both
    /// send and accept, so every projected edge carries `readonly` both ways.
    /// `edge` is the single shared declared role (consent referential
    /// integrity needs a role some member declares).
    fn pairwise_manifest(n: usize) -> CohortManifest {
        assert!(n >= 2, "a mesh needs at least 2 members");
        let members: Vec<CohortMember> = (0..n)
            .map(|i| CohortMember {
                host_id: format!("host-{i}"),
                fingerprint: fp_for(i),
                roles: vec!["edge".to_string()],
            })
            .collect();
        // For each member-as-peer, one send + one accept tuple. The projection
        // filters by counterparty, so every edge toward host-j gets readonly.
        let send: Vec<ConsentTuple> = (0..n)
            .map(|i| ConsentTuple {
                peer: format!("host-{i}"),
                role: "edge".to_string(),
                intent: "readonly".to_string(),
            })
            .collect();
        let accept = send.clone();
        CohortManifest {
            schema_version: SCHEMA_VERSION,
            cohort_id: "task2-mesh".to_string(),
            version: 1,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![pubkey_hex(1)],
            },
            members,
            consent: ConsentMatrix { send, accept },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.to_string(),
                RESERVED_INTENT_HALT_RECEIPT.to_string(),
            ],
            t_stale_secs: T_STALE_DEFAULT,
            signature: ManifestSignature { sig: String::new() },
        }
    }

    #[test]
    fn peer_configs_count_is_n_minus_one_derived() {
        // The edge count is DERIVED from N (members.len() - 1), never a literal
        // — exercised across several N including the Task-2 fleet size of 8.
        for n in [2usize, 3, 5, 8] {
            let m = pairwise_manifest(n).signed_with(&signing_key(1));
            let cfgs = m.peer_configs_for("host-0").expect("peer configs");
            let expected = n - 1;
            assert_eq!(
                cfgs.len(),
                expected,
                "N={n}: a full-pairwise mesh yields N-1={expected} peer edges"
            );
        }
    }

    #[test]
    fn peer_configs_exclude_self() {
        let n = 5;
        let m = pairwise_manifest(n).signed_with(&signing_key(1));
        for i in 0..n {
            let self_host = format!("host-{i}");
            let cfgs = m.peer_configs_for(&self_host).expect("peer configs");
            assert!(
                cfgs.iter().all(|c| c.peer_id.as_str() != self_host),
                "self {self_host} must never appear among its own peer edges"
            );
            assert_eq!(cfgs.len(), n - 1, "N={n} from {self_host}");
        }
    }

    #[test]
    fn peer_configs_carry_declared_fingerprints() {
        let n = 4;
        let m = pairwise_manifest(n).signed_with(&signing_key(1));
        let cfgs = m.peer_configs_for("host-0").expect("peer configs");
        // Each edge's §7.2 fingerprint MUST equal the declared member pin, and
        // the peer set is exactly the non-self members in declaration order.
        for c in &cfgs {
            let declared = m
                .members
                .iter()
                .find(|mem| mem.host_id == c.peer_id.as_str())
                .expect("edge peer must be a declared member");
            let parsed = PeerCertFingerprint::parse(&declared.fingerprint).unwrap();
            assert_eq!(
                c.cert_fingerprint, parsed,
                "edge fingerprint must reconcile with the declared pin for {}",
                c.peer_id
            );
        }
        let got: Vec<String> = cfgs
            .iter()
            .map(|c| c.peer_id.as_str().to_string())
            .collect();
        let want: Vec<String> = (1..n).map(|i| format!("host-{i}")).collect();
        assert_eq!(got, want, "peer edges in declaration order, self excluded");
    }

    #[test]
    fn peer_configs_reject_unknown_host_with_typed_error() {
        let m = pairwise_manifest(3).signed_with(&signing_key(1));
        let err = m.peer_configs_for("host-ghost").unwrap_err();
        assert!(
            matches!(err, CohortError::EHostNotMember { ref host_id } if host_id == "host-ghost"),
            "an unknown self host MUST be rejected with the typed EHostNotMember, got {err:?}"
        );
    }

    #[test]
    fn peer_configs_project_consent_matrix_by_counterparty() {
        // Asymmetric consent: host-0 sends `readonly` TO host-1 and accepts
        // `rca-summary` FROM host-1; NO tuple names host-0, so host-1's
        // reciprocal edge toward host-0 has empty allowlists. This proves the
        // projection filters by counterparty peer, direction-correct.
        let m = CohortManifest {
            schema_version: SCHEMA_VERSION,
            cohort_id: "asym".to_string(),
            version: 1,
            authority: CohortAuthority {
                threshold: 1,
                keys: vec![pubkey_hex(1)],
            },
            members: vec![
                CohortMember {
                    host_id: "host-0".to_string(),
                    fingerprint: fp_for(0),
                    roles: vec!["edge".to_string()],
                },
                CohortMember {
                    host_id: "host-1".to_string(),
                    fingerprint: fp_for(1),
                    roles: vec!["edge".to_string()],
                },
            ],
            consent: ConsentMatrix {
                send: vec![ConsentTuple {
                    peer: "host-1".to_string(),
                    role: "edge".to_string(),
                    intent: "readonly".to_string(),
                }],
                accept: vec![ConsentTuple {
                    peer: "host-1".to_string(),
                    role: "edge".to_string(),
                    intent: "rca-summary".to_string(),
                }],
            },
            reserved_intents: vec![
                RESERVED_INTENT_REISSUE.to_string(),
                RESERVED_INTENT_HALT_RECEIPT.to_string(),
            ],
            t_stale_secs: T_STALE_DEFAULT,
            signature: ManifestSignature { sig: String::new() },
        }
        .signed_with(&signing_key(1));

        // host-0 → host-1: send=[readonly], accept=[rca-summary].
        let cfgs0 = m.peer_configs_for("host-0").unwrap();
        assert_eq!(cfgs0.len(), 1);
        let edge0 = &cfgs0[0];
        assert_eq!(edge0.peer_id.as_str(), "host-1");
        assert_eq!(
            edge0.allowlists.send_allowlist,
            vec![A2AIntent::new("readonly")]
        );
        assert_eq!(
            edge0.allowlists.accept_allowlist,
            vec![A2AIntent::new("rca-summary")]
        );

        // host-1 → host-0: no consent tuple names host-0 → empty allowlists
        // (default-deny), but the edge still exists (host-0 is a member).
        let cfgs1 = m.peer_configs_for("host-1").unwrap();
        assert_eq!(cfgs1.len(), 1);
        assert_eq!(cfgs1[0].peer_id.as_str(), "host-0");
        assert!(cfgs1[0].allowlists.send_allowlist.is_empty());
        assert!(cfgs1[0].allowlists.accept_allowlist.is_empty());
    }

    #[test]
    fn peer_configs_deterministic() {
        // Same manifest + same self host ⇒ byte-identical edges (declaration
        // order, no hashing nondeterminism).
        let m = pairwise_manifest(4).signed_with(&signing_key(1));
        let a = m.peer_configs_for("host-1").unwrap();
        let b = m.peer_configs_for("host-1").unwrap();
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.peer_id, y.peer_id);
            assert_eq!(x.endpoint, y.endpoint);
            assert_eq!(x.cert_fingerprint, y.cert_fingerprint);
            assert_eq!(x.profile, y.profile);
            assert_eq!(x.allowlists, y.allowlists);
        }
    }

    #[test]
    fn peer_configs_are_valid_crosshost_tls_peer_configs() {
        // Projected edges must be valid A2APeerConfig: CrossHost profile, a
        // `tls://` placeholder endpoint that passes validate(), and §7.2
        // defaults for the manifest-absent per-peer knobs.
        let m = pairwise_manifest(3).signed_with(&signing_key(1));
        let cfgs = m.peer_configs_for("host-0").unwrap();
        for c in &cfgs {
            assert_eq!(c.profile, A2AProfile::CrossHost);
            assert!(
                c.endpoint.starts_with("tls://"),
                "endpoint must be tls:// (got {})",
                c.endpoint
            );
            assert_eq!(c.partition_timeout_secs, PEER_PARTITION_TIMEOUT_SECS);
            assert_eq!(c.consent_ttl_secs, PEER_CONSENT_TTL_SECS);
            c.validate()
                .expect("a projected peer config must pass A2APeerConfig::validate");
        }
    }

    #[test]
    fn peer_configs_from_signed_verified_manifest() {
        // End-to-end: a signed manifest whose signature verifies under the
        // pinned genesis authority projects a correct N−1 edge set.
        let m = pairwise_manifest(3).signed_with(&signing_key(1));
        let pinned = pinned_authority();
        m.verify_signature(&pinned)
            .expect("signed manifest verifies under the pinned authority");
        let cfgs = m.peer_configs_for("host-0").expect("peer configs");
        assert_eq!(cfgs.len(), 3 - 1);
    }
}
