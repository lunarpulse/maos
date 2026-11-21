#![forbid(unsafe_code)]

//! Shared-tier memory store — Host-wide SQLite-backed key-value store
//! with namespace prefix per writer (Story 4.3).
//!
//! The `writer_spirit_pid` is kernel-set from the calling context, not
//! Spirit-supplied — this is the I5 enforcement substrate.  Readers
//! query by `(namespace, key)` regardless of writer_pid (shared-tier
//! semantics: "all Spirits on this Host" per §9.1).

use std::path::Path;
use std::sync::Mutex;

use super::{self as quota, TierLimit, Usage};
use maos_domain::memory::{
    MemoryEntry, MemoryError, MemoryNamespace, MemoryTier, MemoryValue, ValueKind,
};
use rusqlite::{params, Connection};

const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS shared_memory (
    writer_spirit_pid INTEGER NOT NULL,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value BLOB NOT NULL,
    kind TEXT NOT NULL,
    timestamp_ns INTEGER NOT NULL,
    writer_owner TEXT
);
CREATE INDEX IF NOT EXISTS shared_memory_namespace_idx ON shared_memory(namespace);
";

/// Uniqueness follows the writer identity, not the boot-local pid: an owned row
/// is keyed by its stable owner, an ownerless legacy row by its pid. Created
/// after the owner-column and pid-key migrations in `open`.
const IDENTITY_INDEX_SQL: &str = "
CREATE UNIQUE INDEX IF NOT EXISTS shared_memory_identity_uidx
    ON shared_memory(COALESCE(writer_owner, 'pid:' || writer_spirit_pid), namespace, key);
";

/// Host-wide SQLite-backed shared memory store.
#[maos_attrs::i9_exempt(
    reason = "memory manager shared tier — Host-wide SQLite-backed kv with namespace prefix per writer for cross-Spirit coordination; bounded by the per-identity tier quota (bytes + entries, refused with MemoryError::QuotaExceeded; FR28) + namespace ownership; kernel writer_spirit_pid is kernel-set, not Spirit-supplied"
)]
pub struct SharedMemoryStore {
    conn: Mutex<Connection>,
    limit: TierLimit,
}

impl SharedMemoryStore {
    /// Open the shared-memory table on the given SQLite file (re-uses the
    /// Transparency Log DB — same file, different table).
    pub fn open(path: &Path) -> Result<Self, MemoryError> {
        let conn = Connection::open(path).map_err(|e| MemoryError::Storage(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        conn.execute_batch(CREATE_TABLE_SQL)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        // Every pre-21.6 table keys rows by (pid, namespace, key), so a different
        // Spirit reusing a pid would overwrite the row: rebuild without that key
        // (the rows stay readable, ownerless and uncharged, and gain the owner column).
        let pid_keyed: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('shared_memory') WHERE pk != 0)",
                [],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        if pid_keyed {
            conn.execute_batch(&format!(
                "BEGIN; ALTER TABLE shared_memory RENAME TO shared_memory_pid_keyed;
                 DROP INDEX IF EXISTS shared_memory_namespace_idx; {CREATE_TABLE_SQL}
                 INSERT INTO shared_memory SELECT writer_spirit_pid, namespace, key, value, kind,
                        timestamp_ns, NULL FROM shared_memory_pid_keyed;
                 DROP TABLE shared_memory_pid_keyed; COMMIT;"
            ))
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        }
        conn.execute_batch(IDENTITY_INDEX_SQL)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
            limit: quota::DEFAULT_SHARED_LIMIT,
        })
    }

    /// Apply the operator-resolved per-identity quota (FR28).
    pub fn with_quota(mut self, limit: TierLimit) -> Self {
        self.limit = limit;
        self
    }

    fn kind_to_str(kind: ValueKind) -> &'static str {
        match kind {
            ValueKind::Json => "json",
            ValueKind::Markdown => "markdown",
            ValueKind::Blob => "blob",
            ValueKind::Text => "text",
        }
    }

    fn kind_from_str(s: &str) -> Result<ValueKind, MemoryError> {
        match s {
            "json" => Ok(ValueKind::Json),
            "markdown" => Ok(ValueKind::Markdown),
            "blob" => Ok(ValueKind::Blob),
            "text" => Ok(ValueKind::Text),
            _ => Err(MemoryError::Storage(format!("unknown kind: {s}"))),
        }
    }

    fn ns_to_str(ns: &MemoryNamespace) -> Result<String, MemoryError> {
        serde_json::to_string(ns)
            .map_err(|e| MemoryError::Storage(format!("namespace serialize: {e}")))
    }

    fn value_bytes(value: &MemoryValue) -> Result<(Vec<u8>, &'static str), MemoryError> {
        match value {
            MemoryValue::Json(v) => serde_json::to_vec(v)
                .map(|b| (b, "json"))
                .map_err(|e| MemoryError::Storage(e.to_string())),
            MemoryValue::Markdown(s) => Ok((s.as_bytes().to_vec(), "markdown")),
            MemoryValue::Blob(b) => Ok((b.clone(), "blob")),
            MemoryValue::Text(s) => Ok((s.as_bytes().to_vec(), "text")),
        }
    }

    fn value_from_row(kind_str: &str, bytes: &[u8]) -> Result<MemoryValue, MemoryError> {
        let kind = Self::kind_from_str(kind_str)?;
        match kind {
            ValueKind::Json => {
                let v: serde_json::Value = serde_json::from_slice(bytes)
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Json(v))
            }
            ValueKind::Markdown => {
                let s = String::from_utf8(bytes.to_vec())
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Markdown(s))
            }
            ValueKind::Blob => Ok(MemoryValue::Blob(bytes.to_vec())),
            ValueKind::Text => {
                let s = String::from_utf8(bytes.to_vec())
                    .map_err(|e| MemoryError::Storage(e.to_string()))?;
                Ok(MemoryValue::Text(s))
            }
        }
    }

    fn now_ns() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    // ------------------------------------------------------------------
    // public API
    // ------------------------------------------------------------------

    /// Write a value for the given writer spirit.  `writer_spirit_pid` is
    /// kernel-set from the calling context; `writer_owner` is its stable
    /// storage identity, which owns the row and the quota charge. A write
    /// replaces that owner's row for `(namespace, key)` whatever pid wrote it.
    /// `enforce = false` charges without refusing (kernel-authored writes).
    pub(in crate::memory) fn write(
        &self,
        writer_spirit_pid: u32,
        writer_owner: &str,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        enforce: bool,
    ) -> Result<(), MemoryError> {
        let (bytes, kind_str) = Self::value_bytes(&value)?;
        let ns_str = Self::ns_to_str(namespace)?;
        let ts = Self::now_ns();
        let db = |e: rusqlite::Error| MemoryError::Storage(e.to_string());

        // The check, the replace and the insert share one transaction under
        // the connection mutex, so concurrent writers cannot both pass.
        let conn = self.conn.lock().expect("SharedMemoryStore lock poisoned");
        let tx = conn.unchecked_transaction().map_err(db)?;
        if enforce {
            let (used, old) = Self::usage(&tx, writer_owner, &ns_str, key)?;
            let new_len = quota::entry_len(key, bytes.len() as u64);
            quota::check(MemoryTier::Shared, self.limit, used, old, new_len)?;
        }
        // `INSERT OR REPLACE` replaces on the identity index: the same owner's row
        // for (namespace, key) whatever pid wrote it, never another owner's.
        tx.execute(
            "INSERT OR REPLACE INTO shared_memory
                (writer_spirit_pid, namespace, key, value, kind, timestamp_ns, writer_owner)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                writer_spirit_pid as i64,
                ns_str,
                key,
                bytes,
                kind_str,
                ts as i64,
                writer_owner,
            ],
        )
        .map_err(db)?;
        tx.commit().map_err(db)
    }

    /// One owner's charge and the existing charge of `(namespace, key)`, summed
    /// from its rows: no ledger, so restarts and failed writes cannot drift.
    fn usage(
        conn: &Connection,
        owner: &str,
        ns: &str,
        key: &str,
    ) -> Result<(Usage, Option<u64>), MemoryError> {
        conn.query_row(
            "SELECT COALESCE(SUM(len), 0), COUNT(*),
                    SUM(CASE WHEN namespace = ?2 AND key = ?3 THEN len END)
             FROM (SELECT namespace, key, length(CAST(key AS BLOB)) + length(value) AS len
                   FROM shared_memory WHERE writer_owner = ?1)",
            params![owner, ns, key],
            |row| {
                let used = Usage {
                    bytes: row.get(0)?,
                    entries: row.get(1)?,
                };
                Ok((used, row.get(2)?))
            },
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))
    }

    /// Delete the caller's own row for `(namespace, key)`; other writers' and
    /// legacy ownerless rows are untouched. Returns whether a row existed.
    pub(in crate::memory) fn delete(
        &self,
        writer_owner: &str,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<bool, MemoryError> {
        let ns_str = Self::ns_to_str(namespace)?;
        let conn = self.conn.lock().expect("SharedMemoryStore lock poisoned");
        conn.execute(
            "DELETE FROM shared_memory WHERE writer_owner = ?1 AND namespace = ?2 AND key = ?3",
            params![writer_owner, ns_str, key],
        )
        .map(|rows| rows > 0)
        .map_err(|e| MemoryError::Storage(e.to_string()))
    }

    /// Read a value from the shared store.  Queries by `(namespace, key)`
    /// regardless of writer_pid — shared-tier semantics.
    pub(in crate::memory) fn read(
        &self,
        _reader_spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, MemoryError> {
        let ns_str = Self::ns_to_str(namespace)?;
        let conn = self.conn.lock().expect("SharedMemoryStore lock poisoned");

        // Return the most recent write when multiple writers share the
        // same namespace+key (deterministic last-write-wins).
        let mut stmt = conn
            .prepare(
                "SELECT value, kind FROM shared_memory
                 WHERE namespace = ?1 AND key = ?2
                 ORDER BY timestamp_ns DESC LIMIT 1",
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let mut rows = stmt
            .query_map(params![ns_str, key], |row| {
                let bytes: Vec<u8> = row.get(0)?;
                let kind_str: String = row.get(1)?;
                Ok((bytes, kind_str))
            })
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        match rows.next() {
            Some(Ok((bytes, kind_str))) => {
                let value = Self::value_from_row(&kind_str, &bytes)?;
                Ok(Some(value))
            }
            Some(Err(e)) => Err(MemoryError::Storage(e.to_string())),
            None => Ok(None),
        }
    }

    /// Scan shared entries matching a key prefix within a namespace.
    pub(in crate::memory) fn scan(
        &self,
        _reader_spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, MemoryError> {
        let ns_str = Self::ns_to_str(namespace)?;
        let conn = self.conn.lock().expect("SharedMemoryStore lock poisoned");

        let mut stmt = conn
            .prepare(
                "SELECT writer_spirit_pid, namespace, key, value, kind, timestamp_ns
                 FROM shared_memory
                 WHERE namespace = ?1 AND key LIKE ?2 ESCAPE '\\'
                 ORDER BY writer_spirit_pid, key
                 LIMIT ?3",
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        // Escape SQL LIKE wildcards in the prefix so they are treated as
        // literal characters.
        let like_pattern = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like_pattern = format!("{like_pattern}%");
        let rows = stmt
            .query_map(params![ns_str, like_pattern, limit as i64], |row| {
                let _writer_pid: i64 = row.get(0)?;
                let namespace_str: String = row.get(1)?;
                let key: String = row.get(2)?;
                let bytes: Vec<u8> = row.get(3)?;
                let kind_str: String = row.get(4)?;
                let ts: i64 = row.get(5)?;
                Ok((namespace_str, key, bytes, kind_str, ts))
            })
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            let (ns_json, key, bytes, kind_str, ts) =
                row.map_err(|e| MemoryError::Storage(e.to_string()))?;
            let value = Self::value_from_row(&kind_str, &bytes)?;
            let namespace: MemoryNamespace = serde_json::from_str(&ns_json)
                .map_err(|e| MemoryError::Storage(format!("namespace deserialize: {e}")))?;
            entries.push(MemoryEntry {
                namespace,
                key,
                value,
                timestamp_ns: ts as u64,
            });
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_store() -> (SharedMemoryStore, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let store = SharedMemoryStore::open(&db_path).unwrap();
        (store, tmp)
    }

    fn put(store: &SharedMemoryStore, pid: u32, ns: &MemoryNamespace, key: &str, v: MemoryValue) {
        store
            .write(pid, &pid.to_string(), ns, key, v, true)
            .unwrap();
    }

    #[test]
    fn write_and_read_same_writer() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Text("shared data".into());
        put(&store, 1, &MemoryNamespace::Coordination, "k1", val.clone());
        let got = store.read(1, &MemoryNamespace::Coordination, "k1").unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn cross_writer_read() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Text("spirit-a data".into());
        put(
            &store,
            10,
            &MemoryNamespace::Coordination,
            "shared-key",
            val.clone(),
        );
        // Spirit-B reads the same key — shared-tier semantics.
        let got = store
            .read(20, &MemoryNamespace::Coordination, "shared-key")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn insert_or_replace_semantics() {
        let (store, _tmp) = make_store();
        for text in ["first", "second"] {
            put(
                &store,
                1,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text(text.into()),
            );
        }
        let got = store.read(1, &MemoryNamespace::Default, "k").unwrap();
        assert_eq!(got, Some(MemoryValue::Text("second".into())));
    }

    #[test]
    fn scan_returns_deterministic_order() {
        let (store, _tmp) = make_store();
        for (pid, key, text) in [(1, "alpha", "a"), (0, "beta", "b"), (1, "gamma", "c")] {
            put(
                &store,
                pid,
                &MemoryNamespace::Coordination,
                key,
                MemoryValue::Text(text.into()),
            );
        }
        let results = store
            .scan(0, &MemoryNamespace::Coordination, "", 10)
            .unwrap();
        // Order by (writer_spirit_pid, key)
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, vec!["beta", "alpha", "gamma"]);
    }

    #[test]
    fn json_roundtrip_via_kind_column() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Json(serde_json::json!({"nested": [1, 2, 3]}));
        put(
            &store,
            1,
            &MemoryNamespace::Default,
            "json-key",
            val.clone(),
        );
        let got = store
            .read(1, &MemoryNamespace::Default, "json-key")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn read_missing_returns_none() {
        let (store, _tmp) = make_store();
        let got = store
            .read(1, &MemoryNamespace::Default, "no-such-key")
            .unwrap();
        assert!(got.is_none());
    }
}
