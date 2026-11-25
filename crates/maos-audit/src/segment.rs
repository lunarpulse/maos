//! Story 22-6 (ADR-072) — sealed audit segments: the `maos.audit-segment.v1` manifest, its
//! signature, its digests, the segment chain and the merged read over segments ∪ live.
//!
//! A segment is a SQLite file holding a prefix of the Transparency Log with its original
//! insertion ids, plus a signed JSON manifest naming the file's sha256, a content digest over
//! every row and side row, the Merkle root over the frame ids, the predecessor manifest's
//! sha256 and (when the Host has one) the region. Signing reuses the sealed-export primitives:
//! canonical JSON, sha256, Ed25519, and the region-derived key. The verifier never reads the
//! `attester_pubkey` the manifest carries (R-RG1).
//!
//! Boundary (RELEASE-HOLDS, "bounded audit store"): a signature proves that the holder of the
//! operator audit key sealed these bytes. It does not prove the rows are complete or that they
//! were written at their stated times, and the chain detects a missing, reordered or forked
//! segment only between segments that are present.

use crate::sealed_export::{
    canonicalize_value, derive_pubkey, derive_region_signing_seed, SignatureBlock,
};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use maos_domain::region::Region;
use rusqlite::types::Value;
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// The manifest schema id `maosctl audit verify-bundle` dispatches on.
pub const SEGMENT_SCHEMA: &str = "maos.audit-segment.v1";
/// Intent of the live-log row journaled before an expired segment is unlinked.
pub const SEGMENT_EXPIRED_INTENT: &str = "audit.segment.expired";
const CONTENT_DOMAIN: &[u8] = b"maos.audit-segment.content.v1\n";

const LOG_COLUMNS: &str =
    "rowid, frame_id, timestamp_ns, spirit_pid, from_spirit_id, to_spirit_id, \
    boot_nonce, capability_token, kind, intent, correlation_id, payload_redacted, origin";
/// `(table, columns, order by)` — the side tables that move with their frames.
const SIDE_TABLES: [(&str, &str, &str); 4] = [
    (
        "transparency_log_retractions",
        "original_frame_id, retract_frame_id, retracted_at_ns",
        "original_frame_id",
    ),
    (
        "transparency_log_recipients",
        "frame_id, recipient_pid, recipient_spirit_id, recipient_role",
        "frame_id, recipient_pid",
    ),
    (
        "transparency_log_consent",
        "frame_id, consent_id, granter_spirit_id, granter_host_id, granter_role, \
         granted_at_key, intent_class, valid_until_key, recall_policy",
        "frame_id",
    ),
    (
        "transparency_log_consent_ruptures",
        "rupture_frame_id, recipient_pid, granter_spirit_id, granter_host_id, intent_class",
        "rupture_frame_id, recipient_pid",
    ),
];

#[derive(Debug, thiserror::Error)]
pub enum SegmentError {
    #[error("i/o: {0}")]
    Io(String),
    #[error("manifest is not valid JSON of the segment schema: {0}")]
    Parse(String),
    #[error("manifest schema_version is `{0}`, not `{SEGMENT_SCHEMA}`")]
    Schema(String),
    #[error("invalid key or signature material: {0}")]
    Key(String),
    #[error("signature verification failed")]
    Signature,
    #[error("segment file `{0}` is missing beside its manifest")]
    MissingFile(String),
    #[error("segment file digest mismatch (manifest {manifest}, file {actual})")]
    FileDigest { manifest: String, actual: String },
    #[error("segment content digest mismatch (manifest {manifest}, recomputed {actual})")]
    ContentDigest { manifest: String, actual: String },
    #[error("segment Merkle root mismatch (manifest {manifest}, recomputed {actual})")]
    MerkleRoot { manifest: String, actual: String },
    #[error("segment shape mismatch: {0}")]
    Shape(String),
    #[error("sqlite: {0}")]
    Sqlite(String),
}

impl From<rusqlite::Error> for SegmentError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}

/// The signed manifest. `signature_block` is absent while signing and verifying (the digest
/// covers every other field, so a tamper of any of them, the region included, breaks it).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SegmentManifest {
    pub schema_version: String,
    pub seq: u64,
    /// The sibling file name, `<seq:010>.sqlite`.
    pub file: String,
    /// Exclusive lower bound of the insertion ids the segment holds.
    pub after_rowid: i64,
    pub first_rowid: i64,
    pub last_rowid: i64,
    pub row_count: u64,
    pub min_timestamp_ns: i64,
    pub max_timestamp_ns: i64,
    pub file_sha256: String,
    pub content_digest: String,
    pub merkle_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_manifest_sha256: Option<String>,
    /// Digests of every earlier manifest of this segment a re-seal replaced, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// Frame ids (hex) a forget scrubbed in this segment, across all re-seals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scrubbed_frame_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_block: Option<SignatureBlock>,
}

/// What `seal_segment` needs beyond the file itself.
#[derive(Debug, Clone)]
pub struct SealParams {
    pub seq: u64,
    pub file_name: String,
    pub after_rowid: i64,
    pub first_rowid: i64,
    pub last_rowid: i64,
    pub row_count: u64,
    pub min_timestamp_ns: i64,
    pub max_timestamp_ns: i64,
    pub prev_manifest_sha256: Option<String>,
    /// The manifest file this seal replaces (a re-seal after a forget).
    pub replaces_manifest: Option<PathBuf>,
    pub scrubbed_frame_ids: Vec<String>,
    pub region: Option<Region>,
    pub host: Option<String>,
}

fn feed(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn feed_value(hasher: &mut Sha256, value: &Value) {
    match value {
        Value::Null => hasher.update([0u8]),
        Value::Integer(n) => {
            hasher.update([1u8]);
            hasher.update(n.to_be_bytes());
        }
        Value::Real(f) => {
            hasher.update([2u8]);
            hasher.update(f.to_bits().to_be_bytes());
        }
        Value::Text(text) => {
            hasher.update([3u8]);
            feed(hasher, text.as_bytes());
        }
        Value::Blob(blob) => {
            hasher.update([4u8]);
            feed(hasher, blob);
        }
    }
}

/// sha256 of a file (hex).
pub fn file_sha256(path: &Path) -> Result<String, SegmentError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)
        .map_err(|e| SegmentError::Io(format!("{}: {e}", path.display())))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| SegmentError::Io(e.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn open_segment_readonly(path: &Path) -> Result<Connection, SegmentError> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_NO_MUTEX
            | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )?)
}

/// The digest over every column of every row (insertion order) and every side row (key order).
pub fn content_digest(conn: &Connection) -> Result<String, SegmentError> {
    let mut hasher = Sha256::new();
    hasher.update(CONTENT_DOMAIN);
    let mut feed_table = |name: &str, columns: &str, order: &str| -> Result<(), SegmentError> {
        feed(&mut hasher, name.as_bytes());
        let mut statement =
            conn.prepare(&format!("SELECT {columns} FROM {name} ORDER BY {order}"))?;
        let width = statement.column_count();
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            for index in 0..width {
                feed_value(&mut hasher, &row.get::<_, Value>(index)?);
            }
        }
        Ok(())
    };
    feed_table("transparency_log", LOG_COLUMNS, "rowid")?;
    for (table, columns, order) in SIDE_TABLES {
        feed_table(table, columns, order)?;
    }
    Ok(hex::encode(hasher.finalize()))
}

/// The Merkle root over the segment's frame ids (all-zero for an empty segment).
pub fn merkle_root(conn: &Connection) -> Result<String, SegmentError> {
    let mut statement = conn.prepare("SELECT frame_id FROM transparency_log ORDER BY frame_id")?;
    let ids = statement
        .query_map([], |row| {
            let blob: Vec<u8> = row.get(0)?;
            <[u8; 16]>::try_from(blob.as_slice()).map_err(|_| {
                rusqlite::Error::InvalidParameterName(format!(
                    "frame_id must be 16 bytes, got {}",
                    blob.len()
                ))
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    if ids.is_empty() {
        return Ok(hex::encode([0u8; 32]));
    }
    Ok(hex::encode(
        crate::erasure::merkle::build_tree_from_frame_ids(&ids).root,
    ))
}

fn signing_seed(seed: &[u8; 32], region: Option<&str>) -> Result<[u8; 32], SegmentError> {
    match region {
        Some(tag) => {
            let region = Region::canonicalize(tag)
                .map_err(|e| SegmentError::Key(format!("invalid region tag: {e}")))?;
            Ok(derive_region_signing_seed(seed, &region))
        }
        None => Ok(*seed),
    }
}

/// The key that must have signed a manifest claiming `region`, from the operator base seed.
pub fn expected_pubkey(seed: &[u8; 32], region: Option<&str>) -> Result<[u8; 32], SegmentError> {
    Ok(derive_pubkey(&signing_seed(seed, region)?))
}

fn signed_bytes(manifest: &SegmentManifest) -> Result<[u8; 32], SegmentError> {
    let mut unsigned = manifest.clone();
    unsigned.signature_block = None;
    let canonical =
        canonicalize_value(&unsigned).map_err(|e| SegmentError::Parse(e.to_string()))?;
    Ok(Sha256::digest(canonical).into())
}

/// Digest the finished segment file, sign the manifest, and return its bytes and sha256.
pub fn seal_segment(
    segment_path: &Path,
    params: &SealParams,
    seed: &[u8; 32],
) -> Result<(Vec<u8>, String), SegmentError> {
    let conn = open_segment_readonly(segment_path)?;
    let mut supersedes = Vec::new();
    let mut scrubbed: std::collections::BTreeSet<String> =
        params.scrubbed_frame_ids.iter().cloned().collect();
    if let Some(old_path) = &params.replaces_manifest {
        let old_bytes = std::fs::read(old_path)
            .map_err(|e| SegmentError::Io(format!("{}: {e}", old_path.display())))?;
        let old: SegmentManifest =
            serde_json::from_slice(&old_bytes).map_err(|e| SegmentError::Parse(e.to_string()))?;
        supersedes = old.supersedes;
        supersedes.push(sha256_hex(&old_bytes));
        scrubbed.extend(old.scrubbed_frame_ids);
    }
    let mut manifest = SegmentManifest {
        schema_version: SEGMENT_SCHEMA.to_string(),
        seq: params.seq,
        file: params.file_name.clone(),
        after_rowid: params.after_rowid,
        first_rowid: params.first_rowid,
        last_rowid: params.last_rowid,
        row_count: params.row_count,
        min_timestamp_ns: params.min_timestamp_ns,
        max_timestamp_ns: params.max_timestamp_ns,
        file_sha256: file_sha256(segment_path)?,
        content_digest: content_digest(&conn)?,
        merkle_root: merkle_root(&conn)?,
        prev_manifest_sha256: params.prev_manifest_sha256.clone(),
        supersedes,
        scrubbed_frame_ids: scrubbed.into_iter().collect(),
        region: params.region.as_ref().map(|r| r.as_str().to_string()),
        host: params.host.clone(),
        signature_block: None,
    };
    sign_manifest(&mut manifest, seed)?;
    let bytes =
        serde_json::to_vec_pretty(&manifest).map_err(|e| SegmentError::Parse(e.to_string()))?;
    let sha = sha256_hex(&bytes);
    Ok((bytes, sha))
}

/// Sign `manifest` over every field but the signature, with the base `seed` (the key is
/// region-derived when the manifest claims a region).
pub fn sign_manifest(manifest: &mut SegmentManifest, seed: &[u8; 32]) -> Result<(), SegmentError> {
    let digest = signed_bytes(manifest)?;
    let key = SigningKey::from_bytes(&signing_seed(seed, manifest.region.as_deref())?);
    manifest.signature_block = Some(SignatureBlock {
        algorithm: "Ed25519".to_string(),
        attester_pubkey: hex::encode(key.verifying_key().to_bytes()),
        signature: hex::encode(key.sign(&digest).to_bytes()),
    });
    Ok(())
}

/// Parse a manifest, refusing any other schema.
pub fn parse_manifest(bytes: &[u8]) -> Result<SegmentManifest, SegmentError> {
    let manifest: SegmentManifest =
        serde_json::from_slice(bytes).map_err(|e| SegmentError::Parse(e.to_string()))?;
    if manifest.schema_version != SEGMENT_SCHEMA {
        return Err(SegmentError::Schema(manifest.schema_version));
    }
    Ok(manifest)
}

/// The signature check alone: `pubkey` must be the key that signs for the manifest's claimed region.
pub fn verify_manifest_signature(
    manifest: &SegmentManifest,
    pubkey: &[u8; 32],
) -> Result<(), SegmentError> {
    let verifying =
        VerifyingKey::from_bytes(pubkey).map_err(|e| SegmentError::Key(e.to_string()))?;
    let block = manifest
        .signature_block
        .as_ref()
        .ok_or_else(|| SegmentError::Key("manifest carries no signature_block".into()))?;
    let signature: [u8; 64] = hex::decode(&block.signature)
        .map_err(|e| SegmentError::Key(format!("signature hex: {e}")))?
        .try_into()
        .map_err(|v: Vec<u8>| {
            SegmentError::Key(format!("signature must be 64 bytes, got {}", v.len()))
        })?;
    verifying
        .verify(
            &signed_bytes(manifest)?,
            &ed25519_dalek::Signature::from_bytes(&signature),
        )
        .map_err(|_| SegmentError::Signature)
}

/// Verify one segment from its manifest and sibling file: the signature, then the file digest,
/// the content digest, the Merkle root and the claimed shape recomputed from the file.
pub fn verify_segment(
    manifest_path: &Path,
    pubkey: &[u8; 32],
) -> Result<SegmentManifest, SegmentError> {
    let bytes = std::fs::read(manifest_path)
        .map_err(|e| SegmentError::Io(format!("{}: {e}", manifest_path.display())))?;
    let manifest = parse_manifest(&bytes)?;
    verify_manifest_signature(&manifest, pubkey)?;
    verify_segment_file(manifest_path, &manifest)?;
    Ok(manifest)
}

/// Everything except the signature (also what `verify-segments` runs without a key).
pub fn verify_segment_file(
    manifest_path: &Path,
    manifest: &SegmentManifest,
) -> Result<(), SegmentError> {
    let dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    if manifest.file.contains('/') || manifest.file.contains('\\') || manifest.file.contains("..") {
        return Err(SegmentError::Shape(format!(
            "file name `{}` is not a sibling",
            manifest.file
        )));
    }
    let file = dir.join(&manifest.file);
    if !file.is_file() {
        return Err(SegmentError::MissingFile(manifest.file.clone()));
    }
    let actual = file_sha256(&file)?;
    if actual != manifest.file_sha256 {
        return Err(SegmentError::FileDigest {
            manifest: manifest.file_sha256.clone(),
            actual,
        });
    }
    let conn = open_segment_readonly(&file)?;
    let content = content_digest(&conn)?;
    if content != manifest.content_digest {
        return Err(SegmentError::ContentDigest {
            manifest: manifest.content_digest.clone(),
            actual: content,
        });
    }
    let root = merkle_root(&conn)?;
    if root != manifest.merkle_root {
        return Err(SegmentError::MerkleRoot {
            manifest: manifest.merkle_root.clone(),
            actual: root,
        });
    }
    let (rows, first, last, min_ts, max_ts): (i64, i64, i64, i64, i64) = conn.query_row(
        "SELECT count(*), coalesce(min(rowid),0), coalesce(max(rowid),0), \
         coalesce(min(timestamp_ns),0), coalesce(max(timestamp_ns),0) FROM transparency_log",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    if rows as u64 != manifest.row_count
        || first != manifest.first_rowid
        || last != manifest.last_rowid
        || min_ts != manifest.min_timestamp_ns
        || max_ts != manifest.max_timestamp_ns
        || first <= manifest.after_rowid
    {
        return Err(SegmentError::Shape(format!(
            "file holds {rows} rows over ids {first}..={last}; manifest claims {} rows over {}..={} after {}",
            manifest.row_count, manifest.first_rowid, manifest.last_rowid, manifest.after_rowid
        )));
    }
    Ok(())
}

// ── the chain ───────────────────────────────────────────────────────────────

/// One `audit.segment.expired` row of the live log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredSegment {
    pub seq: u64,
    pub manifest_sha256: String,
}

/// The expiry rows a Transparency Log journaled (they explain a gap in the chain).
pub fn expired_segments(db_path: &Path) -> Result<Vec<ExpiredSegment>, crate::AuditError> {
    // Lenient on purpose: this reader exists to explain a chain with holes, so a registered
    // segment that is missing must not stop it from reading the rows that do exist.
    let mut found = Vec::new();
    for conn in open_sources(db_path, false)? {
        let mut statement = conn
            .prepare("SELECT payload_redacted FROM transparency_log WHERE intent = ?1")
            .map_err(crate::AuditError::Read)?;
        let payloads = statement
            .query_map([SEGMENT_EXPIRED_INTENT], |row| row.get::<_, Vec<u8>>(0))
            .map_err(crate::AuditError::Read)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::AuditError::Read)?;
        found.extend(payloads.iter().filter_map(|payload| {
            let payload: serde_json::Value = serde_json::from_slice(payload).ok()?;
            Some(ExpiredSegment {
                seq: payload.get("seq")?.as_u64()?,
                manifest_sha256: payload.get("manifest_sha256")?.as_str()?.replace(':', ""),
            })
        }));
    }
    Ok(found)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainReport {
    /// Manifests found in the directory.
    pub segments: usize,
    /// Expired segments the live log explains.
    pub explained_gaps: usize,
    pub problems: Vec<String>,
}

/// Check the chain of `<seq>.manifest.json` files in `dir`: each file digest, contiguous insertion
/// ranges, each manifest naming its predecessor (directly or through `supersedes`), no fork, no
/// reorder. A gap (including a missing head) is explained only by an `expired` row. When
/// `key_for` yields a key for a manifest its signature and digests are verified too.
pub fn verify_chain(
    dir: &Path,
    expired: &[ExpiredSegment],
    key_for: &dyn Fn(&SegmentManifest) -> Option<[u8; 32]>,
) -> ChainReport {
    let mut problems = Vec::new();
    let mut found: Vec<(PathBuf, Vec<u8>, SegmentManifest)> = Vec::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            let mut paths: Vec<PathBuf> = entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.ends_with(".manifest.json"))
                })
                .collect();
            paths.sort();
            for path in paths {
                let name = path.display().to_string();
                match std::fs::read(&path) {
                    Ok(bytes) => match parse_manifest(&bytes) {
                        Ok(manifest) => found.push((path, bytes, manifest)),
                        Err(error) => problems.push(format!("{name}: {error}")),
                    },
                    Err(error) => problems.push(format!("{name}: {error}")),
                }
            }
        }
        Err(error) => problems.push(format!("{}: {error}", dir.display())),
    }
    found.sort_by_key(|(_, _, manifest)| manifest.seq);
    let expired_seqs: std::collections::BTreeMap<u64, &str> = expired
        .iter()
        .map(|e| (e.seq, e.manifest_sha256.as_str()))
        .collect();
    let mut explained = 0usize;
    for (path, _, manifest) in &found {
        let seq = manifest.seq;
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_none_or(|n| n != format!("{seq:010}.manifest.json"))
        {
            problems.push(format!(
                "{}: manifest names seq {seq}, not its file name",
                path.display()
            ));
        }
        match key_for(manifest) {
            Some(key) => {
                if let Err(error) = verify_manifest_signature(manifest, &key)
                    .and_then(|()| verify_segment_file(path, manifest))
                {
                    problems.push(format!("segment {seq}: {error}"));
                }
            }
            None => {
                if let Err(error) = verify_segment_file(path, manifest) {
                    problems.push(format!("segment {seq}: {error}"));
                }
            }
        }
    }
    let mut previous: Option<&(PathBuf, Vec<u8>, SegmentManifest)> = None;
    let mut missing = |from: u64,
                       to: u64,
                       successor_prev: Option<&str>,
                       problems: &mut Vec<String>| {
        // Segments `from..to` are absent: each needs an expiry row, and the last one's digest
        // must be what the successor names as its predecessor.
        for seq in from..to {
            match expired_seqs.get(&seq) {
                Some(digest) => {
                    explained += 1;
                    if seq + 1 == to && successor_prev.is_some_and(|prev| prev != *digest) {
                        problems.push(format!(
                            "segment {seq}: the expiry row names a manifest the successor does not chain to"
                        ));
                    }
                }
                None => problems.push(format!(
                    "segment {seq} is missing and no audit.segment.expired row explains it"
                )),
            }
        }
    };
    for entry in &found {
        let manifest = &entry.2;
        match previous {
            None => {
                if manifest.seq > 1 {
                    missing(
                        1,
                        manifest.seq,
                        manifest.prev_manifest_sha256.as_deref(),
                        &mut problems,
                    );
                } else if manifest.prev_manifest_sha256.is_some() {
                    problems.push(format!(
                        "segment {}: the first segment names a predecessor",
                        manifest.seq
                    ));
                }
            }
            Some(before) => {
                let earlier = &before.2;
                if manifest.seq == earlier.seq {
                    problems.push(format!(
                        "segment {}: two manifests claim this sequence (fork)",
                        manifest.seq
                    ));
                } else {
                    if manifest.seq > earlier.seq + 1 {
                        missing(
                            earlier.seq + 1,
                            manifest.seq,
                            manifest.prev_manifest_sha256.as_deref(),
                            &mut problems,
                        );
                    } else {
                        let digest = sha256_hex(&before.1);
                        let chained = manifest.prev_manifest_sha256.as_deref()
                            == Some(digest.as_str())
                            || manifest
                                .prev_manifest_sha256
                                .as_ref()
                                .is_some_and(|prev| earlier.supersedes.contains(prev));
                        if !chained {
                            problems.push(format!(
                                "segment {}: does not chain to segment {} (forked or reordered)",
                                manifest.seq, earlier.seq
                            ));
                        }
                    }
                    if manifest.after_rowid != earlier.last_rowid && manifest.seq == earlier.seq + 1
                    {
                        problems.push(format!(
                            "segment {}: insertion ids are not contiguous with segment {} ({} != {})",
                            manifest.seq, earlier.seq, manifest.after_rowid, earlier.last_rowid
                        ));
                    }
                    if manifest.after_rowid < earlier.last_rowid {
                        problems.push(format!(
                            "segment {}: insertion ids overlap segment {}",
                            manifest.seq, earlier.seq
                        ));
                    }
                }
            }
        }
        previous = Some(entry);
    }
    ChainReport {
        segments: found.len(),
        explained_gaps: explained,
        problems,
    }
}

// ── merged reads ────────────────────────────────────────────────────────────

/// Every readable source of one Transparency Log, oldest first: the registered, unexpired sealed
/// segments (named by the live file's registry), then the live file. A registered segment that
/// is missing is an error, never a silently shorter log.
pub fn open_log_sources(db_path: &Path) -> Result<Vec<Connection>, crate::AuditError> {
    open_sources(db_path, true)
}

/// As [`open_log_sources`], but every source is opened `immutable=1`: SQLite never touches a
/// `-wal`/`-shm` sidecar, so a read leaves nothing beside a quiesced or snapshot database.
/// Only valid when nothing writes the files while they are read (a checkpointed snapshot).
pub fn open_log_sources_immutable(db_path: &Path) -> Result<Vec<Connection>, crate::AuditError> {
    open_sources_with(db_path, true, true, false)
}

fn open_sources(db_path: &Path, strict: bool) -> Result<Vec<Connection>, crate::AuditError> {
    open_sources_with(db_path, strict, false, false)
}

/// Every source of a log whose live file a daemon may be writing: the live file through a
/// NORMAL read-only connection (never `immutable=1`) inside ONE read transaction that the
/// caller ends (`COMMIT`) as soon as its bounded read is done, plus the quiesced sealed
/// segments (immutable). The last connection is the live file.
pub fn open_log_sources_live(db_path: &Path) -> Result<Vec<Connection>, crate::AuditError> {
    open_sources_with(db_path, true, false, true)
}

/// A normal (never `immutable=1`) read-only connection to a live file.
pub fn open_ro_live(path: &Path) -> rusqlite::Result<Connection> {
    open_ro(path, false)
}

fn open_ro(path: &Path, immutable: bool) -> rusqlite::Result<Connection> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    if !immutable {
        return Connection::open_with_flags(path, flags);
    }
    let mut uri = String::from("file:");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => {
                uri.push(byte as char)
            }
            other => uri.push_str(&format!("%{other:02X}")),
        }
    }
    uri.push_str("?immutable=1");
    Connection::open_with_flags(uri, flags | OpenFlags::SQLITE_OPEN_URI)
}

fn open_sources_with(
    db_path: &Path,
    strict: bool,
    immutable: bool,
    live_tx: bool,
) -> Result<Vec<Connection>, crate::AuditError> {
    let live = open_ro(db_path, immutable).map_err(crate::AuditError::Open)?;
    if live_tx {
        // One read transaction on the LIVE file, begun before the registry is read: the
        // registry, the live rows and the segment list all come from one snapshot, so a
        // rotation between two of our statements cannot hide rows from us.
        live.execute_batch("BEGIN")
            .map_err(crate::AuditError::Read)?;
    }
    let registered: bool = live
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' \
             AND name='transparency_log_segments')",
            [],
            |row| row.get(0),
        )
        .map_err(crate::AuditError::Read)?;
    let mut sources = Vec::new();
    if registered {
        let mut statement = live
            .prepare("SELECT seq FROM transparency_log_segments WHERE expired_at_ns IS NULL ORDER BY seq")
            .map_err(crate::AuditError::Read)?;
        let seqs = statement
            .query_map([], |row| row.get::<_, i64>(0))
            .map_err(crate::AuditError::Read)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(crate::AuditError::Read)?;
        drop(statement);
        let dir = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("segments");
        for seq in seqs {
            let path = dir.join(format!("{seq:010}.sqlite"));
            // Sealed segments are quiesced: always immutable when the live file is read live.
            match open_ro(&path, immutable || live_tx) {
                Ok(conn) => sources.push(conn),
                Err(_) if !strict => {}
                Err(error) => {
                    return Err(crate::AuditError::Row(format!(
                        "sealed segment {seq} is unavailable at {}: {error}",
                        path.display()
                    )))
                }
            }
        }
    }
    sources.push(live);
    Ok(sources)
}

/// Count and bytes of the sealed segments a log directory holds (`maos purge --keep-log`).
pub fn segment_footprint(db_path: &Path) -> (u64, u64) {
    let dir = db_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("segments");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    let mut count = 0;
    let mut bytes = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".sqlite") {
            count += 1;
        }
        if name.ends_with(".sqlite") || name.ends_with(".manifest.json") {
            bytes += entry.metadata().map_or(0, |m| m.len());
        }
    }
    (count, bytes)
}
