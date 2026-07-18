#![forbid(unsafe_code)]

//! Canonical serialization for a collective-memory KV row (Story 11.2a AC3).
//!
//! Mirrors the engine-independent design of `canonical.rs`: every row reduces
//! to a fixed, pinned byte layout so that two regions hashing the SAME logical
//! row produce the IDENTICAL digest, and the convergence oracle triple (payload
//! oracle + Merkle root) agrees by construction across regions.
//!
//! # Canonical form (pinned — never change without an ADR)
//!
//! Each collective-memory KV row reduces to this byte layout:
//!
//! domain               26 bytes (b"maos.collective-kv-leaf.v1", verbatim)
//! source_region_len    4 bytes (big-endian u32) + <len> UTF-8 bytes
//! source_ts            8 bytes (big-endian i64)
//! spirit_pid           8 bytes (big-endian i64)
//! namespace_kind_len   4 bytes (big-endian u32) + <len> UTF-8 bytes
//! namespace_detail_len 4 bytes (big-endian u32) + <len> UTF-8 bytes
//! key_len              4 bytes (big-endian u32) + <len> UTF-8 bytes
//! value_kind_len       4 bytes (big-endian u32) + <len> UTF-8 bytes
//! value_data_len       4 bytes (big-endian u32) + <len> raw bytes
//! source_team_len      v2 ONLY: 4 bytes (big-endian u32) + <len> UTF-8 bytes
//!                      (v1 / None-team leaves omit this field ENTIRELY and use
//!                       the v1 domain tag, so their bytes are unchanged)
//! ```
//!
//! ## Encoding rules (pinned)
//!
//! - **Domain separator** (`KV_LEAF_DOMAIN`): written verbatim at the front to
//!   namespace this leaf shape from every other canonical form (notably the
//!   Transparency-Log `CanonicalFrame`). It is a fixed constant, so no length
//!   prefix is needed.
//! - **Integers** (`source_ts`, `spirit_pid`): big-endian fixed-width i64.
//! - **Variable-length text/bytes fields**: UTF-8 (or raw) bytes verbatim with
//!   a 4-byte big-endian length prefix on EVERY field. The length prefix is
//!   load-bearing: it makes a boundary shift such as `region="ab",key="c"`
//!   vs `region="a",key="bc"` canonically distinct.

use maos_domain::team::TeamId;
use sha2::{Digest, Sha256};

/// Domain separator namespaceing the collective-KV leaf canonical form from
/// every other canonical byte layout (notably the TL `CanonicalFrame`).
const KV_LEAF_DOMAIN: &[u8] = b"maos.collective-kv-leaf.v1";
/// Story 13.2 (AC2): v2 domain tag for a leaf carrying `source_team`. The v2
/// body is the v1 field order followed by a length-prefixed `source_team`. A
/// `None`-team leaf stays under [`KV_LEAF_DOMAIN`] (byte-identical to pre-13.2);
/// only a `Some`-team leaf uses this tag. The domain-tag axis and the
/// bundle `schema_version` axis are INDEPENDENT version axes (Yui) — bumping one
/// never implies the other.
const KV_LEAF_DOMAIN_V2: &[u8] = b"maos.collective-kv-leaf.v2";

/// A single collective-memory KV row reduced to the engine-independent
/// canonical form used by the cross-region convergence oracle and replication.
///
/// Field set mirrors `crate::store::CollectiveRow` MINUS `source_log_ref`,
/// which is a provenance reference that is intentionally NOT part of the
/// convergence hash (two regions may reference different local log entries for
/// the same logical row and must still converge).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectiveKvLeaf {
    pub source_region: String,
    pub source_ts: i64,
    pub spirit_pid: i64,
    pub namespace_kind: String,
    pub namespace_detail: String,
    pub key: String,
    pub value_kind: String,
    pub value_data: Vec<u8>,
    /// Story 13.2 (AC2): source-team provenance. `None` for first-party local
    /// rows (byte-identical v1 leaves — the 9.2b additive idiom); `Some` only
    /// for re-attested cross-team copies (built by 13.3 / synthesized in tests).
    /// `serde(default)` keeps a v1 bundle (no `source_team` in the wire) able to
    /// deserialize; `skip_serializing_if` keeps the v1 wire byte-clean.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_team: Option<TeamId>,
}

impl CollectiveKvLeaf {
    /// Construct a leaf from a raw collective-memory row.
    pub fn from_row(row: &crate::store::CollectiveRow) -> Self {
        Self {
            source_region: row.source_region.clone(),
            source_ts: row.source_ts,
            spirit_pid: row.spirit_pid,
            namespace_kind: row.namespace_kind.clone(),
            namespace_detail: row.namespace_detail.clone(),
            key: row.key.clone(),
            value_kind: row.value_kind.clone(),
            value_data: row.value_data.clone(),
            source_team: row.source_team.clone(),
        }
    }

    /// Serialize this KV row into the pinned canonical byte form.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        // Dispatch on source_team (AC2). `None` → the EXACT pre-13.2 v1 layout
        // under KV_LEAF_DOMAIN, byte-for-byte identical (11.2a cross-region
        // convergence for existing rows depends on this). `Some` → KV_LEAF_DOMAIN_V2,
        // the identical v1 field order, then a length-prefixed source_team appended
        // at the tail. The length prefix keeps source_team boundary-shift-safe.
        let (domain, team_bytes): (&[u8], Option<&[u8]>) = match &self.source_team {
            None => (KV_LEAF_DOMAIN, None),
            Some(team) => (KV_LEAF_DOMAIN_V2, Some(team.as_str().as_bytes())),
        };
        let mut buf = Vec::with_capacity(
            domain.len()
                + 4 + self.source_region.len()
                + 8 // source_ts
                + 8 // spirit_pid
                + 4 + self.namespace_kind.len()
                + 4 + self.namespace_detail.len()
                + 4 + self.key.len()
                + 4 + self.value_kind.len()
                + 4 + self.value_data.len()
                + team_bytes.map_or(0, |t| 4 + t.len()),
        );
        buf.extend_from_slice(domain);
        write_lp_bytes(&mut buf, self.source_region.as_bytes());
        buf.extend_from_slice(&self.source_ts.to_be_bytes());
        buf.extend_from_slice(&self.spirit_pid.to_be_bytes());
        write_lp_bytes(&mut buf, self.namespace_kind.as_bytes());
        write_lp_bytes(&mut buf, self.namespace_detail.as_bytes());
        write_lp_bytes(&mut buf, self.key.as_bytes());
        write_lp_bytes(&mut buf, self.value_kind.as_bytes());
        write_lp_bytes(&mut buf, &self.value_data);
        if let Some(team) = team_bytes {
            write_lp_bytes(&mut buf, team);
        }
        buf
    }

    /// SHA-256 of the canonical form — the per-row contribution to the
    /// payload oracle and the Merkle leaf hash.
    pub fn canonical_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.to_canonical_bytes());
        hasher.finalize().into()
    }
}

/// Append a 4-byte big-endian length prefix followed by the bytes.
fn write_lp_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// Compute the payload oracle: SHA-256 over the sorted multiset of per-row
/// canonical hashes.
///
/// Mirrors `canonical::compute_payload_oracle`. Because every field is part of
/// the canonical hash, this oracle catches ANY single-byte mutation in ANY
/// column that the Merkle root (deduplicated over equal hashes) is blind to.
pub fn compute_kv_payload_oracle(leaves: &[CollectiveKvLeaf]) -> [u8; 32] {
    let mut row_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| l.canonical_hash()).collect();
    row_hashes.sort_unstable();
    let mut hasher = Sha256::new();
    for h in &row_hashes {
        hasher.update(h);
    }
    hasher.finalize().into()
}

/// Merkle root over the per-leaf canonical hashes.
///
/// Empty set → `[0u8; 32]`, mirroring `canonical::merkle_root_from_frame_ids`
/// and `maos_audit::backup::compute_merkle_root` exactly (B14 empty sentinel).
/// Otherwise delegates to `maos_audit::erasure::merkle::build_tree`, which
/// sorts + dedups the leaf hashes before reducing to the root.
pub fn kv_merkle_root(leaves: &[CollectiveKvLeaf]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let leaf_hashes: Vec<[u8; 32]> = leaves.iter().map(|l| l.canonical_hash()).collect();
    maos_audit::erasure::merkle::build_tree(&leaf_hashes).root
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_leaf() -> CollectiveKvLeaf {
        CollectiveKvLeaf {
            source_region: "us-west-2a".to_string(),
            source_ts: 1_700_000_000_000_000_000,
            spirit_pid: 42,
            namespace_kind: "local".to_string(),
            namespace_detail: "detail-x".to_string(),
            key: "memory-key".to_string(),
            value_kind: "text/plain".to_string(),
            value_data: b"hello-world".to_vec(),
            source_team: None,
        }
    }

    #[test]
    fn test_canonical_bytes_deterministic() {
        let a = sample_leaf().to_canonical_bytes();
        let b = sample_leaf().to_canonical_bytes();
        assert_eq!(a, b, "identical leaves must canonicalize identically");
    }

    #[test]
    fn test_different_field_different_hash() {
        let base = sample_leaf();

        // region_only_diff
        let mut region_only_diff = base.clone();
        region_only_diff.source_region = "us-west-2b".to_string();
        assert_ne!(
            base.canonical_hash(),
            region_only_diff.canonical_hash(),
            "region_only_diff must change the hash"
        );

        // ts_only_diff
        let mut ts_only_diff = base.clone();
        ts_only_diff.source_ts = base.source_ts + 1;
        assert_ne!(
            base.canonical_hash(),
            ts_only_diff.canonical_hash(),
            "ts_only_diff must change the hash"
        );

        // value_only_diff
        let mut value_only_diff = base.clone();
        value_only_diff.value_data = b"different-value".to_vec();
        assert_ne!(
            base.canonical_hash(),
            value_only_diff.canonical_hash(),
            "value_only_diff must change the hash"
        );

        // spirit_pid_only_diff
        let mut spirit_pid_only_diff = base.clone();
        spirit_pid_only_diff.spirit_pid = base.spirit_pid + 1;
        assert_ne!(
            base.canonical_hash(),
            spirit_pid_only_diff.canonical_hash(),
            "spirit_pid_only_diff must change the hash"
        );

        // namespace_kind_only_diff
        let mut namespace_kind_only_diff = base.clone();
        namespace_kind_only_diff.namespace_kind = "global".to_string();
        assert_ne!(
            base.canonical_hash(),
            namespace_kind_only_diff.canonical_hash(),
            "namespace_kind_only_diff must change the hash"
        );

        // namespace_detail_only_diff (EVERY field coverage)
        let mut namespace_detail_only_diff = base.clone();
        namespace_detail_only_diff.namespace_detail = "other-detail".to_string();
        assert_ne!(
            base.canonical_hash(),
            namespace_detail_only_diff.canonical_hash(),
            "namespace_detail_only_diff must change the hash"
        );

        // key_only_diff
        let mut key_only_diff = base.clone();
        key_only_diff.key = "other-key".to_string();
        assert_ne!(
            base.canonical_hash(),
            key_only_diff.canonical_hash(),
            "key_only_diff must change the hash"
        );

        // value_kind_only_diff (EVERY field coverage)
        let mut value_kind_only_diff = base.clone();
        value_kind_only_diff.value_kind = "application/json".to_string();
        assert_ne!(
            base.canonical_hash(),
            value_kind_only_diff.canonical_hash(),
            "value_kind_only_diff must change the hash"
        );
    }

    #[test]
    fn test_boundary_shift_no_collision() {
        // The length prefix is load-bearing: without it, concatenating
        // region + ... + key could let "ab"|"c" collide with "a"|"bc".
        let mut a = sample_leaf();
        a.source_region = "ab".to_string();
        a.key = "c".to_string();
        let mut b = sample_leaf();
        b.source_region = "a".to_string();
        b.key = "bc".to_string();
        assert_ne!(
            a.canonical_hash(),
            b.canonical_hash(),
            "boundary shift across source_region/key MUST NOT collide (length prefix is load-bearing)"
        );
    }

    #[test]
    fn test_empty_set_root() {
        assert_eq!(kv_merkle_root(&[]), [0u8; 32], "empty leaf set → zero root");
    }

    #[test]
    fn test_payload_oracle_sorted() {
        let mut l1 = sample_leaf();
        l1.key = "k1".to_string();
        let mut l2 = sample_leaf();
        l2.key = "k2".to_string();
        let order_a = vec![l1.clone(), l2.clone()];
        let order_b = vec![l2, l1];
        assert_eq!(
            compute_kv_payload_oracle(&order_a),
            compute_kv_payload_oracle(&order_b),
            "oracle must be order-independent (sorted multiset)"
        );
    }

    #[test]
    fn test_stub_returning_empty_cannot_pass() {
        let mut l1 = sample_leaf();
        l1.key = "stub-a".to_string();
        let mut l2 = sample_leaf();
        l2.key = "stub-b".to_string();
        let h1 = l1.canonical_hash();
        let h2 = l2.canonical_hash();
        assert_ne!(
            h1, [0u8; 32],
            "canonical_hash must not be the all-zero stub digest"
        );
        assert_ne!(
            h2, [0u8; 32],
            "canonical_hash must not be the all-zero stub digest"
        );
        assert_ne!(
            h1, h2,
            "distinct leaves must yield distinct non-empty digests"
        );
    }

    #[test]
    fn from_row_roundtrips_all_hashed_fields() {
        let row = crate::store::CollectiveRow {
            spirit_pid: 7,
            namespace_kind: "nk".to_string(),
            namespace_detail: "nd".to_string(),
            key: "k".to_string(),
            value_kind: "vk".to_string(),
            value_data: b"vd".to_vec(),
            source_region: "eu-central-1".to_string(),
            source_ts: 123,
            source_log_ref: "log-ref-not-hashed".to_string(),
            source_team: None,
        };
        let leaf = CollectiveKvLeaf::from_row(&row);
        assert_eq!(leaf.source_region, "eu-central-1");
        assert_eq!(leaf.source_ts, 123);
        assert_eq!(leaf.spirit_pid, 7);
        assert_eq!(leaf.namespace_kind, "nk");
        assert_eq!(leaf.namespace_detail, "nd");
        assert_eq!(leaf.key, "k");
        assert_eq!(leaf.value_kind, "vk");
        assert_eq!(leaf.value_data, b"vd");
    }

    #[test]
    fn test_empty_fields_produce_distinct_hashes() {
        // Edge case: empty strings in every variable-length field must still
        // produce a valid (non-zero) hash and be distinct from the sample leaf.
        let empty = CollectiveKvLeaf {
            source_region: String::new(),
            source_ts: 0,
            spirit_pid: 0,
            namespace_kind: String::new(),
            namespace_detail: String::new(),
            key: String::new(),
            value_kind: String::new(),
            value_data: Vec::new(),
            source_team: None,
        };
        let h = empty.canonical_hash();
        assert_ne!(h, [0u8; 32], "empty leaf must NOT produce all-zero hash");
        assert_ne!(
            h,
            sample_leaf().canonical_hash(),
            "empty leaf must differ from the sample"
        );
    }

    #[test]
    fn test_max_i64_values() {
        // Edge case: maximum integer values should not overflow or panic.
        let leaf = CollectiveKvLeaf {
            source_region: "r".to_string(),
            source_ts: i64::MAX,
            spirit_pid: i64::MAX,
            namespace_kind: "k".to_string(),
            namespace_detail: "d".to_string(),
            key: "k".to_string(),
            value_kind: "v".to_string(),
            value_data: vec![0xff; 1024],
            source_team: None,
        };
        let h = leaf.canonical_hash();
        assert_ne!(h, [0u8; 32], "max-value leaf must not produce zero hash");
    }

    #[test]
    fn test_merkle_root_single_leaf() {
        let leaf = sample_leaf();
        let root = kv_merkle_root(&[leaf.clone()]);
        assert_ne!(root, [0u8; 32], "single-leaf root must be non-zero");
        // Deterministic: same leaf → same root.
        assert_eq!(
            root,
            kv_merkle_root(&[leaf]),
            "merkle root must be deterministic"
        );
    }

    #[test]
    fn test_payload_oracle_detects_single_byte_in_value() {
        // The payload oracle (not just the Merkle root) must catch a one-byte
        // difference in value_data.
        let l1 = sample_leaf();
        let mut l2 = sample_leaf();
        l2.value_data = b"hello-worlD".to_vec(); // 'd' → 'D'
        assert_ne!(
            compute_kv_payload_oracle(&[l1]),
            compute_kv_payload_oracle(&[l2]),
            "payload oracle must detect a single-byte value mutation"
        );
    }

    #[test]
    fn test_source_log_ref_excluded_from_hash() {
        // source_log_ref is intentionally excluded from the canonical hash
        // (two regions may have different local log refs for the same row).
        let row1 = crate::store::CollectiveRow {
            spirit_pid: 1,
            namespace_kind: "default".to_string(),
            namespace_detail: String::new(),
            key: "k".to_string(),
            value_kind: "text".to_string(),
            value_data: b"v".to_vec(),
            source_region: "r".to_string(),
            source_ts: 100,
            source_log_ref: "ref-A".to_string(),
            source_team: None,
        };
        let row2 = crate::store::CollectiveRow {
            source_log_ref: "ref-B".to_string(),
            ..row1.clone()
        };
        let leaf1 = CollectiveKvLeaf::from_row(&row1);
        let leaf2 = CollectiveKvLeaf::from_row(&row2);
        assert_eq!(
            leaf1.canonical_hash(),
            leaf2.canonical_hash(),
            "source_log_ref must NOT affect the canonical hash"
        );
    }

    // ─── Story 13.2 AC2 — leaf v2 (source_team) ──────────────────────────────

    /// The write codec's oracle triple over a leaf set.
    fn triple(leaves: &[CollectiveKvLeaf]) -> ([u8; 32], [u8; 32], usize) {
        (
            kv_merkle_root(leaves),
            compute_kv_payload_oracle(leaves),
            leaves.len(),
        )
    }

    #[test]
    fn test_v1_canonical_hash_golden() {
        // FROZEN GOLDEN (AC2): a None-team (v1) leaf MUST reproduce the exact
        // pre-13.2 canonical hash byte-for-byte. If this reds, leaf v2 changed
        // v1 bytes → 11.2a cross-region convergence for every existing row
        // breaks. Pinned at 13.2; changing it is an ADR-gated re-key.
        assert_eq!(
            hex::encode(sample_leaf().canonical_hash()),
            "22fe823721e83979508088bfc4e5dac4acfabe264e98c4e97abb25a8a13c2a00",
            "v1 (None-team) canonical hash drifted — v1 leaves are NOT byte-identical"
        );
    }

    #[test]
    fn test_source_team_v2_differs_from_v1() {
        // A Some-team leaf uses the v2 domain tag + appended source_team, so its
        // hash MUST differ from the byte-identical None-team v1 leaf.
        let v1 = sample_leaf();
        let mut v2 = sample_leaf();
        v2.source_team = Some(TeamId::new("team-a").unwrap());
        assert_ne!(
            v1.canonical_hash(),
            v2.canonical_hash(),
            "a source_team-bearing v2 leaf must not collide with its v1 form"
        );
        // Distinct teams → distinct hashes (source_team is in the pre-image).
        let mut v2b = sample_leaf();
        v2b.source_team = Some(TeamId::new("team-b").unwrap());
        assert_ne!(
            v2.canonical_hash(),
            v2b.canonical_hash(),
            "distinct source_team must change the canonical hash"
        );
    }

    #[test]
    fn test_source_team_boundary_shift_no_collision() {
        // The v2 length prefix on source_team is load-bearing: team="ab",key="c"
        // must not collide with team="a",key="bc" (a boundary shift).
        let mut a = sample_leaf();
        a.source_team = Some(TeamId::new("ab").unwrap());
        a.key = "c".to_string();
        let mut b = sample_leaf();
        b.source_team = Some(TeamId::new("a-").unwrap());
        b.key = "bc".to_string();
        assert_ne!(
            a.canonical_hash(),
            b.canonical_hash(),
            "source_team/key boundary shift MUST NOT collide (length prefix load-bearing)"
        );
    }

    #[test]
    fn test_mixed_v1_v2_independence_and_convergence() {
        // AC4 / Round-2 (Murat): a team store holds its own v1 rows (source_team
        // = None) mixed with re-attested v2 copies (source_team = Some). Prove
        // per-team independence and cross-region convergence hold across the mix,
        // and that the payload oracle (not the dedup-blind SET-root) catches a
        // within-team single-byte mutation.
        let team_c = TeamId::new("team-c").unwrap();
        let mut a1 = sample_leaf();
        a1.key = "a-own-1".to_string();
        let mut a2 = sample_leaf();
        a2.key = "a-own-2".to_string();
        let mut a_copy = sample_leaf();
        a_copy.key = "c-copy".to_string();
        a_copy.source_team = Some(team_c.clone());
        let team_a = vec![a1.clone(), a2.clone(), a_copy.clone()];

        let mut b1 = sample_leaf();
        b1.key = "b-own-1".to_string();
        let team_b = vec![b1];

        let a_before = triple(&team_a);

        // Mutate + grow team-B; its root MUST move, team-A's triple MUST NOT.
        let mut b_mut = team_b.clone();
        b_mut[0].value_data = b"mutated".to_vec();
        let mut b_extra = sample_leaf();
        b_extra.key = "b-own-2".to_string();
        b_mut.push(b_extra);
        assert_ne!(
            triple(&team_b).0,
            triple(&b_mut).0,
            "team-B mutation must move team-B's root"
        );
        assert_eq!(
            a_before,
            triple(&team_a),
            "team-A's triple must be independent of any team-B mutation"
        );

        // Convergence: two regions' converged copies are the SAME SET regardless
        // of arrival order → identical root AND oracle.
        let reordered = vec![a_copy, a1, a2];
        assert_eq!(
            a_before.0,
            triple(&reordered).0,
            "mixed-set root order-independent"
        );
        assert_eq!(
            a_before.1,
            triple(&reordered).1,
            "mixed-set oracle order-independent"
        );

        // The payload oracle catches a single-byte value mutation on any leaf.
        // (This mutation also moves the root, since the leaf-hash SET changes —
        // it is NOT the root-blind case; it just confirms the oracle is live.)
        let mut a_tampered = team_a.clone();
        a_tampered[2].value_data = b"hello-worlD".to_vec();
        assert_ne!(
            a_before.1,
            triple(&a_tampered).1,
            "payload oracle must catch a within-team byte mutation of the v2 copy"
        );

        // The 11.2a L3 discipline (payload_byte_flip_changes_oracle_not_root):
        // the SET-root DEDUPS leaf hashes, so it is BLIND to duplicate-COUNT
        // drift — adding a second copy of an existing leaf leaves the root
        // UNCHANGED while the payload oracle AND the row-count both move. A
        // triple that checked only the root would miss this; prove all three.
        let mut a_dup = team_a.clone();
        a_dup.push(team_a[2].clone()); // duplicate of an existing leaf
        let dup_triple = triple(&a_dup);
        assert_eq!(
            a_before.0, dup_triple.0,
            "SET-root is dedup-blind: a duplicate leaf must NOT move the root"
        );
        assert_ne!(
            a_before.1, dup_triple.1,
            "payload oracle MUST catch the duplicate-count drift the root misses"
        );
        assert_ne!(
            a_before.2, dup_triple.2,
            "row-count MUST catch the duplicate-count drift the root misses"
        );
    }
}
