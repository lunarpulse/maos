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
//! ```text
//! domain               27 bytes (b"maos.collective-kv-leaf.v1", verbatim)
//! source_region_len    4 bytes (big-endian u32) + <len> UTF-8 bytes
//! source_ts            8 bytes (big-endian i64)
//! spirit_pid           8 bytes (big-endian i64)
//! namespace_kind_len   4 bytes (big-endian u32) + <len> UTF-8 bytes
//! namespace_detail_len 4 bytes (big-endian u32) + <len> UTF-8 bytes
//! key_len              4 bytes (big-endian u32) + <len> UTF-8 bytes
//! value_kind_len       4 bytes (big-endian u32) + <len> UTF-8 bytes
//! value_data_len       4 bytes (big-endian u32) + <len> raw bytes
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

use sha2::{Digest, Sha256};

/// Domain separator namespaceing the collective-KV leaf canonical form from
/// every other canonical byte layout (notably the TL `CanonicalFrame`).
const KV_LEAF_DOMAIN: &[u8] = b"maos.collective-kv-leaf.v1";

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
        }
    }

    /// Serialize this KV row into the pinned canonical byte form.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(
            KV_LEAF_DOMAIN.len()
                + 4 + self.source_region.len()
                + 8 // source_ts
                + 8 // spirit_pid
                + 4 + self.namespace_kind.len()
                + 4 + self.namespace_detail.len()
                + 4 + self.key.len()
                + 4 + self.value_kind.len()
                + 4 + self.value_data.len(),
        );
        buf.extend_from_slice(KV_LEAF_DOMAIN);
        write_lp_bytes(&mut buf, self.source_region.as_bytes());
        buf.extend_from_slice(&self.source_ts.to_be_bytes());
        buf.extend_from_slice(&self.spirit_pid.to_be_bytes());
        write_lp_bytes(&mut buf, self.namespace_kind.as_bytes());
        write_lp_bytes(&mut buf, self.namespace_detail.as_bytes());
        write_lp_bytes(&mut buf, self.key.as_bytes());
        write_lp_bytes(&mut buf, self.value_kind.as_bytes());
        write_lp_bytes(&mut buf, &self.value_data);
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
}
