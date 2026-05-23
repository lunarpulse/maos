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

use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue, ValueKind};
use rusqlite::{params, Connection};

const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS shared_memory (
    writer_spirit_pid INTEGER NOT NULL,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value BLOB NOT NULL,
    kind TEXT NOT NULL,
    timestamp_ns INTEGER NOT NULL,
    PRIMARY KEY (writer_spirit_pid, namespace, key)
);
CREATE INDEX IF NOT EXISTS shared_memory_namespace_idx ON shared_memory(namespace);
";

/// Host-wide SQLite-backed shared memory store.
#[maos_attrs::i9_exempt(
    reason = "memory manager shared tier — Host-wide SQLite-backed kv with namespace prefix per writer for cross-Spirit coordination; bounded by Spirit lifetime + namespace ownership; kernel writer_spirit_pid is kernel-set, not Spirit-supplied"
)]
pub struct SharedMemoryStore {
    conn: Mutex<Connection>,
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
        Ok(Self {
            conn: Mutex::new(conn),
        })
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

    /// Write a value for the given writer spirit.  The `writer_spirit_pid`
    /// is kernel-set from the calling context.
    pub fn write(
        &self,
        writer_spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), MemoryError> {
        let (bytes, kind_str) = Self::value_bytes(&value)?;
        let ns_str = Self::ns_to_str(namespace)?;
        let ts = Self::now_ns();

        let conn = self.conn.lock().expect("SharedMemoryStore lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO shared_memory (writer_spirit_pid, namespace, key, value, kind, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                writer_spirit_pid as i64,
                ns_str,
                key,
                bytes,
                kind_str,
                ts as i64,
            ],
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Read a value from the shared store.  Queries by `(namespace, key)`
    /// regardless of writer_pid — shared-tier semantics.
    pub fn read(
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
    pub fn scan(
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

    #[test]
    fn write_and_read_same_writer() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Text("shared data".into());
        store
            .write(1, &MemoryNamespace::Coordination, "k1", val.clone())
            .unwrap();
        let got = store.read(1, &MemoryNamespace::Coordination, "k1").unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn cross_writer_read() {
        let (store, _tmp) = make_store();
        let val = MemoryValue::Text("spirit-a data".into());
        store
            .write(
                10,
                &MemoryNamespace::Coordination,
                "shared-key",
                val.clone(),
            )
            .unwrap();
        // Spirit-B reads the same key — shared-tier semantics.
        let got = store
            .read(20, &MemoryNamespace::Coordination, "shared-key")
            .unwrap();
        assert_eq!(got, Some(val));
    }

    #[test]
    fn insert_or_replace_semantics() {
        let (store, _tmp) = make_store();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text("first".into()),
            )
            .unwrap();
        store
            .write(
                1,
                &MemoryNamespace::Default,
                "k",
                MemoryValue::Text("second".into()),
            )
            .unwrap();
        let got = store.read(1, &MemoryNamespace::Default, "k").unwrap();
        assert_eq!(got, Some(MemoryValue::Text("second".into())));
    }

    #[test]
    fn scan_returns_deterministic_order() {
        let (store, _tmp) = make_store();
        store
            .write(
                1,
                &MemoryNamespace::Coordination,
                "alpha",
                MemoryValue::Text("a".into()),
            )
            .unwrap();
        store
            .write(
                0,
                &MemoryNamespace::Coordination,
                "beta",
                MemoryValue::Text("b".into()),
            )
            .unwrap();
        store
            .write(
                1,
                &MemoryNamespace::Coordination,
                "gamma",
                MemoryValue::Text("c".into()),
            )
            .unwrap();
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
        store
            .write(1, &MemoryNamespace::Default, "json-key", val.clone())
            .unwrap();
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
