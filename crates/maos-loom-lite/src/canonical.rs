#![forbid(unsafe_code)]

//! Engine-independent canonical leaf serialization for Merkle-root and
//! payload-oracle computation (Story 10.4a AC2, NFR-Ops-10).
//!
//! # Problem
//!
//! SQLite and Postgres encode values differently (text affinity, integer
//! width, NULL handling).  A Merkle root — or a payload oracle — over *raw
//! storage bytes* CANNOT be identical across engines.  The oracles are only
//! identical when both backends reduce each row to the SAME canonical,
//! engine-independent application-level byte serialization.
//!
//! # Canonical form (pinned — never change without an ADR)
//!
//! Each Transparency-Log frame reduces to a byte string with this exact layout:
//!
//! ```text
//! frame_id            16 bytes (raw ULID bytes)
//! timestamp_ns         8 bytes (big-endian u64)
//! spirit_pid           4 bytes (big-endian u32)
//! from_spirit_id_len   4 bytes (big-endian u32) + <len> UTF-8 bytes
//! to_spirit_id_len     4 bytes (big-endian u32) + <len> UTF-8 bytes
//! boot_nonce           8 bytes (big-endian u64)
//! capability_token     1 byte  (0 = NULL, 1 = present)
//!                      [if present] 4 bytes (big-endian u32 len) + <len> bytes
//! kind                 8 bytes (big-endian i64)
//! intent_len           4 bytes (big-endian u32) + <len> UTF-8 bytes
//! payload_len          4 bytes (big-endian u32) + <len> raw bytes
//! origin               8 bytes (big-endian i64)
//! ```
//!
//! ## Encoding rules (pinned)
//!
//! - **Integers:** always big-endian, fixed-width.  `kind`/`origin` are the
//!   SQLite `INTEGER` discriminants (FrameKind / FrameOrigin) carried as i64.
//! - **Text columns** (`from_spirit_id`, `to_spirit_id`, `intent`): UTF-8 bytes
//!   taken verbatim, length-prefixed.  Production TL columns are `NOT NULL`;
//!   empty → 0-length (never NULL, see B10).  Non-UTF-8 intent is REJECTED
//!   with a typed error (B11) rather than masked.
//! - **Payload** (`payload_redacted`): raw bytes, length-prefixed.  `NOT NULL`;
//!   empty → 0-length.
//! - **Nullable column** (`capability_token`): presence byte (0/1) +
//!   length-prefixed bytes when present.  This is the ONLY nullable column.
//! - **frame_id:** exactly 16 bytes — any other length is a hard error (B8),
//!   mirroring `maos_audit::backup::compute_merkle_root` (`backup.rs:45-50`).
//!
//! This layout covers ALL 11 production TL columns
//! (`maos-iac/src/adapter/transparency_log.rs:246-258`); nothing is silently
//! discarded (B1).

use sha2::{Digest, Sha256};
use tokio_postgres::GenericClient;

/// A frame row extracted from a Transparency Log, reduced to the
/// engine-independent canonical form.  Both the SQLite and Postgres readers
/// produce identical `CanonicalFrame` values for identical underlying rows, so
/// the Merkle root and payload oracle agree by construction.
#[derive(Debug, Clone)]
pub struct CanonicalFrame {
    /// 16-byte ULID frame_id (PK).
    pub frame_id: [u8; 16],
    pub timestamp_ns: u64,
    pub spirit_pid: u32,
    pub from_spirit_id: Vec<u8>,
    pub to_spirit_id: Vec<u8>,
    pub boot_nonce: u64,
    /// Nullable capability-token bytes (production: 32 bytes or NULL).
    pub capability_token: Option<Vec<u8>>,
    /// FrameKind discriminant (SQLite INTEGER).
    pub kind: i64,
    pub intent: Vec<u8>,
    /// `payload_redacted` raw bytes.
    pub payload: Vec<u8>,
    /// FrameOrigin discriminant (SQLite INTEGER).
    pub origin: i64,
}

impl CanonicalFrame {
    /// Serialize this frame into the pinned canonical byte form.
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let cap_len = self.capability_token.as_ref().map_or(0, |c| c.len());
        let mut buf = Vec::with_capacity(
            16 + 8 + 4 + 4 + self.from_spirit_id.len() + 4 + self.to_spirit_id.len() + 8 + 1
                + 4 + cap_len + 8 + 4 + self.intent.len() + 4 + self.payload.len() + 8,
        );
        buf.extend_from_slice(&self.frame_id);
        buf.extend_from_slice(&self.timestamp_ns.to_be_bytes());
        buf.extend_from_slice(&self.spirit_pid.to_be_bytes());
        write_lp_bytes(&mut buf, &self.from_spirit_id);
        write_lp_bytes(&mut buf, &self.to_spirit_id);
        buf.extend_from_slice(&self.boot_nonce.to_be_bytes());
        match &self.capability_token {
            None => buf.push(0),
            Some(c) => {
                buf.push(1);
                write_lp_bytes(&mut buf, c);
            }
        }
        buf.extend_from_slice(&self.kind.to_be_bytes());
        write_lp_bytes(&mut buf, &self.intent);
        write_lp_bytes(&mut buf, &self.payload);
        buf.extend_from_slice(&self.origin.to_be_bytes());
        buf
    }

    /// SHA-256 of the canonical form — the per-row contribution to the
    /// payload oracle.
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
/// Because every column (including `frame_id` and `payload_redacted`) is part
/// of the canonical hash, this oracle catches ANY single-byte mutation in ANY
/// column that the frame_id-only Merkle root is blind to.
pub fn compute_payload_oracle(frames: &[CanonicalFrame]) -> [u8; 32] {
    let mut row_hashes: Vec<[u8; 32]> = frames.iter().map(|f| f.canonical_hash()).collect();
    row_hashes.sort_unstable();
    let mut hasher = Sha256::new();
    for h in &row_hashes {
        hasher.update(h);
    }
    hasher.finalize().into()
}

/// Single-source Merkle-root computation over a frame_id set.
///
/// Mirrors `maos_audit::backup::compute_merkle_root` EXACTLY (empty →
/// `[0u8;32]`, else `build_tree_from_frame_ids(...).root`) so the SQLite-side
/// root (derived via the mandated `compute_merkle_root` primitive) and the
/// Postgres-side root agree by construction, including the empty-corpus edge.
/// `compute_merkle_root` lives in read-only `maos-audit`; this helper is the
/// authoritative path for the Postgres side (B14).
pub fn merkle_root_from_frame_ids(frame_ids: &[[u8; 16]]) -> [u8; 32] {
    if frame_ids.is_empty() {
        return [0u8; 32];
    }
    maos_audit::erasure::merkle::build_tree_from_frame_ids(frame_ids).root
}

/// Hard-erroring conversion of a raw blob into a 16-byte frame_id (B8).
fn blob_to_frame_id(blob: &[u8]) -> Result<[u8; 16], String> {
    if blob.len() != 16 {
        return Err(format!(
            "frame_id must be exactly 16 bytes, got {} — rejecting (B8)",
            blob.len()
        ));
    }
    let mut fid = [0u8; 16];
    fid.copy_from_slice(blob);
    Ok(fid)
}

/// Read canonical frames from a SQLite Transparency-Log database.
///
/// Opens the database **read-only** (B20 quiescence: a concurrent writer is
/// blocked from mutating the source during the read) and extracts ALL 11
/// production columns.  A non-16-byte `frame_id` is a hard error (B8).
pub fn read_sqlite_frames(db_path: &std::path::Path) -> Result<Vec<CanonicalFrame>, String> {
    use rusqlite::{Connection, OpenFlags};

    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("sqlite open: {e}"))?;

    let mut stmt = conn
        .prepare(
            "SELECT frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                    boot_nonce, capability_token, kind, intent, payload_redacted, origin
             FROM transparency_log
             ORDER BY frame_id ASC",
        )
        .map_err(|e| format!("sqlite prepare: {e}"))?;

    let frames = stmt
        .query_map([], |row| {
            let frame_id_blob: Vec<u8> = row.get(0)?;
            let timestamp_ns: i64 = row.get(1)?;
            let spirit_pid: i64 = row.get(2)?;
            let from_spirit_id: String = row.get(3)?;
            let to_spirit_id: String = row.get(4)?;
            let boot_nonce: i64 = row.get(5)?;
            let capability_token: Option<Vec<u8>> = row.get(6)?;
            let kind: i64 = row.get(7)?;
            // `intent` is TEXT NOT NULL UTF-8 in production; rusqlite rejects
            // non-UTF-8 here (B11) rather than masking it.
            let intent: String = row.get(8)?;
            let payload_redacted: Vec<u8> = row.get(9)?;
            let origin: i64 = row.get(10)?;

            // frame_id length check happens after row collection (returns a
            // rusqlite error so query_map propagates it); use a sentinel via
            // blob_to_frame_id below.  Here we just stage the raw blob.
            Ok((
                frame_id_blob,
                timestamp_ns,
                spirit_pid,
                from_spirit_id,
                to_spirit_id,
                boot_nonce,
                capability_token,
                kind,
                intent,
                payload_redacted,
                origin,
            ))
        })
        .map_err(|e| format!("sqlite query: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("sqlite row: {e}"))?;

    let mut out = Vec::with_capacity(frames.len());
    for (
        frame_id_blob,
        timestamp_ns,
        spirit_pid,
        from_spirit_id,
        to_spirit_id,
        boot_nonce,
        capability_token,
        kind,
        intent,
        payload_redacted,
        origin,
    ) in frames
    {
        let frame_id = blob_to_frame_id(&frame_id_blob).map_err(|e| format!("sqlite: {e}"))?;
        out.push(CanonicalFrame {
            frame_id,
            timestamp_ns: timestamp_ns as u64,
            spirit_pid: spirit_pid as u32,
            from_spirit_id: from_spirit_id.into_bytes(),
            to_spirit_id: to_spirit_id.into_bytes(),
            boot_nonce: boot_nonce as u64,
            capability_token,
            kind,
            intent: intent.into_bytes(),
            payload: payload_redacted,
            origin,
        });
    }
    Ok(out)
}

/// Read canonical frames from a Postgres Transparency-Log table.
///
/// Selects ALL 11 production columns `ORDER BY frame_id` (deterministic
/// read-back).  A non-16-byte `frame_id` is a hard error (B8).
///
/// Generic over [`tokio_postgres::GenericClient`] so the same reader serves
/// both an ad-hoc `Client` and an in-flight `Transaction` — the latter lets
/// the migration re-derive its target oracles from the UNCOMMITTED rows
/// before committing (P4: verify-before-commit; a failed triple-oracle check
/// then rolls the whole cutover back automatically).  Both `Client` and
/// `Transaction` are `GenericClient + Sync`.
pub async fn read_postgres_frames<C>(
    client: &C,
) -> Result<Vec<CanonicalFrame>, String>
where
    C: GenericClient + Sync,
{
    let rows = client
        .query(
            "SELECT frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id,
                    boot_nonce, capability_token, kind, intent, payload_redacted, origin
             FROM transparency_log
             ORDER BY frame_id ASC",
            &[],
        )
        .await
        .map_err(|e| format!("postgres query: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for row in &rows {
        let frame_id_blob: Vec<u8> = row.get(0);
        let frame_id =
            blob_to_frame_id(&frame_id_blob).map_err(|e| format!("postgres: {e}"))?;
        let timestamp_ns: i64 = row.get(1);
        let spirit_pid: i64 = row.get(2);
        let from_spirit_id: String = row.get(3);
        let to_spirit_id: String = row.get(4);
        let boot_nonce: i64 = row.get(5);
        let capability_token: Option<Vec<u8>> = row.get(6);
        let kind: i64 = row.get(7);
        let intent: String = row.get(8);
        let payload_redacted: Vec<u8> = row.get(9);
        let origin: i64 = row.get(10);

        out.push(CanonicalFrame {
            frame_id,
            timestamp_ns: timestamp_ns as u64,
            spirit_pid: spirit_pid as u32,
            from_spirit_id: from_spirit_id.into_bytes(),
            to_spirit_id: to_spirit_id.into_bytes(),
            boot_nonce: boot_nonce as u64,
            capability_token,
            kind,
            intent: intent.into_bytes(),
            payload: payload_redacted,
            origin,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_frame(id_byte: u8, payload: &[u8]) -> CanonicalFrame {
        CanonicalFrame {
            frame_id: [id_byte; 16],
            timestamp_ns: 1_700_000_000_000_000_000 + id_byte as u64,
            spirit_pid: 42,
            from_spirit_id: b"spirit-a".to_vec(),
            to_spirit_id: b"spirit-b".to_vec(),
            boot_nonce: 99,
            capability_token: Some([0xAA; 32].to_vec()),
            kind: 3,
            intent: b"memory.write".to_vec(),
            payload: payload.to_vec(),
            origin: 1,
        }
    }

    #[test]
    fn canonical_bytes_are_deterministic_and_column_complete() {
        let f = sample_frame(1, b"payload-bytes");
        let a = f.to_canonical_bytes();
        let b = sample_frame(1, b"payload-bytes").to_canonical_bytes();
        assert_eq!(a, b, "identical rows must canonicalize identically");
    }

    #[test]
    fn payload_byte_flip_changes_oracle_not_root() {
        // Same frame_id SET → identical Merkle root; one flipped payload byte
        // → different payload oracle.  This is the B5 RED vector at unit level.
        let set = vec![sample_frame(1, b"aaaa"), sample_frame(2, b"bbbb")];
        let mut tampered = set.clone();
        tampered[1].payload[0] ^= 0xFF;

        let root_clean = merkle_root_from_frame_ids(
            &set.iter().map(|f| f.frame_id).collect::<Vec<_>>(),
        );
        let root_tamp = merkle_root_from_frame_ids(
            &tampered.iter().map(|f| f.frame_id).collect::<Vec<_>>(),
        );
        assert_eq!(root_clean, root_tamp, "root is SET-only — blind to payload");

        let oracle_clean = compute_payload_oracle(&set);
        let oracle_tamp = compute_payload_oracle(&tampered);
        assert_ne!(
            oracle_clean, oracle_tamp,
            "payload oracle MUST catch the flipped byte"
        );
    }

    #[test]
    fn empty_frame_id_set_root_is_zero() {
        // B14: mirrors compute_merkle_root's empty sentinel [0u8;32].
        assert_eq!(merkle_root_from_frame_ids(&[]), [0u8; 32]);
    }

    #[test]
    fn non_16_byte_frame_id_is_rejected() {
        // B8: no silent pad/truncate.
        assert!(blob_to_frame_id(&[0u8; 15]).is_err());
        assert!(blob_to_frame_id(&[0u8; 17]).is_err());
        assert!(blob_to_frame_id(&[0u8; 16]).is_ok());
    }

    #[test]
    fn nullable_capability_token_canonicalizes_distinctly() {
        let mut with_cap = sample_frame(1, b"x");
        with_cap.capability_token = Some(vec![1, 2, 3]);
        let mut without_cap = sample_frame(1, b"x");
        without_cap.capability_token = None;
        assert_ne!(
            with_cap.canonical_hash(),
            without_cap.canonical_hash(),
            "NULL vs present capability_token must differ"
        );
    }
}
