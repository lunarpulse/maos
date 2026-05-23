#![forbid(unsafe_code)]

//! Principal Namespace Index — kernel-side address-only index of
//! `principal:<principal_id>:<schema>` writes per ADR-026 (Story 4.3).
//!
//! The index makes subject-access query O(log N) on `principal_id`
//! without scanning the private store.  Carries NO content — only the
//! addressing tuple `(principal_id, writer_spirit_pid, schema, key)`.

use std::path::Path;
use std::sync::Mutex;

use maos_domain::memory::{MemoryError, PrincipalIndexRow};
use rusqlite::{params, Connection};

const CREATE_TABLE_SQL: &str = "
CREATE TABLE IF NOT EXISTS principal_index (
    principal_id TEXT NOT NULL,
    writer_spirit_pid INTEGER NOT NULL,
    schema TEXT NOT NULL,
    key TEXT NOT NULL,
    timestamp_ns INTEGER NOT NULL,
    PRIMARY KEY (principal_id, writer_spirit_pid, schema, key)
);
CREATE INDEX IF NOT EXISTS principal_index_id_idx ON principal_index(principal_id);
";

/// Kernel-side address-only index for ADR-026 principal namespace
/// lifecycle operations (subject-access, forget, redaction-on-export).
#[maos_attrs::i9_exempt(
    reason = "principal namespace index — kernel-side address-only index of principal:<id>:<schema> writes for ADR-026 subject-access query + GDPR Art. 17 forget cascade; bounded by principal forget cascade; NO content interpretation per §4.0.7"
)]
pub struct PrincipalNamespaceIndex {
    conn: Mutex<Connection>,
}

impl PrincipalNamespaceIndex {
    /// Open the principal-index table on the given SQLite file (re-uses
    /// the same DB as the Transparency Log + SharedMemoryStore).
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

    /// Record a principal-namespace write — called by `MemoryManagerAdapter::write`
    /// whenever `namespace == MemoryNamespace::Principal { .. }`.
    pub fn record_write(
        &self,
        principal_id: &str,
        writer_spirit_pid: u32,
        schema: &str,
        key: &str,
        timestamp_ns: u64,
    ) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO principal_index
                (principal_id, writer_spirit_pid, schema, key, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                principal_id,
                writer_spirit_pid as i64,
                schema,
                key,
                timestamp_ns as i64,
            ],
        )
        .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Subject-access query — returns every `(writer_spirit_pid, schema,
    /// key, timestamp_ns)` indexed for the given `principal_id` across
    /// ALL Spirits on this Host.  Sorted by `(writer_spirit_pid, schema, key)`.
    pub fn lookup(&self, principal_id: &str) -> Result<Vec<PrincipalIndexRow>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");

        let mut stmt = conn
            .prepare(
                "SELECT principal_id, writer_spirit_pid, schema, key, timestamp_ns
                 FROM principal_index
                 WHERE principal_id = ?1
                 ORDER BY writer_spirit_pid, schema, key",
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![principal_id], |row| {
                let pid: String = row.get(0)?;
                let wsp: i64 = row.get(1)?;
                let wsp_u32 = u32::try_from(wsp).map_err(|_| {
                    rusqlite::Error::InvalidColumnType(
                        1,
                        "writer_spirit_pid".into(),
                        rusqlite::types::Type::Integer,
                    )
                })?;
                let schema: String = row.get(2)?;
                let key: String = row.get(3)?;
                let ts: i64 = row.get(4)?;
                Ok(PrincipalIndexRow {
                    principal_id: pid,
                    writer_spirit_pid: wsp_u32,
                    schema,
                    key,
                    timestamp_ns: ts as u64,
                })
            })
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|e| MemoryError::Storage(e.to_string()))?);
        }
        Ok(entries)
    }

    /// Forget all index rows for a given `principal_id` — returns the
    /// deleted row count.
    pub fn forget(&self, principal_id: &str) -> Result<u64, MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");
        let deleted = conn
            .execute(
                "DELETE FROM principal_index WHERE principal_id = ?1",
                params![principal_id],
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_index() -> (PrincipalNamespaceIndex, TempDir) {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("test.db");
        let idx = PrincipalNamespaceIndex::open(&db_path).unwrap();
        (idx, tmp)
    }

    #[test]
    fn write_and_lookup() {
        let (idx, _tmp) = make_index();
        let ts = PrincipalNamespaceIndex::now_ns();

        idx.record_write("alice@example.org", 10, "calendar", "event-001", ts)
            .unwrap();
        idx.record_write("alice@example.org", 10, "calendar", "event-002", ts)
            .unwrap();
        idx.record_write("alice@example.org", 20, "tasks", "task-001", ts)
            .unwrap();

        let rows = idx.lookup("alice@example.org").unwrap();
        assert_eq!(rows.len(), 3);
        // First two should be from pid=10.
        assert_eq!(rows[0].writer_spirit_pid, 10);
        assert_eq!(rows[1].writer_spirit_pid, 10);
        assert_eq!(rows[2].writer_spirit_pid, 20);
    }

    #[test]
    fn lookup_nonexistent_returns_empty() {
        let (idx, _tmp) = make_index();
        let rows = idx.lookup("bob@example.org").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn forget_then_lookup_empty() {
        let (idx, _tmp) = make_index();
        let ts = PrincipalNamespaceIndex::now_ns();

        idx.record_write("alice@example.org", 10, "calendar", "e1", ts)
            .unwrap();
        idx.record_write("alice@example.org", 10, "calendar", "e2", ts)
            .unwrap();
        idx.record_write("alice@example.org", 20, "tasks", "t1", ts)
            .unwrap();

        let deleted = idx.forget("alice@example.org").unwrap();
        assert_eq!(deleted, 3);

        let rows = idx.lookup("alice@example.org").unwrap();
        assert!(rows.is_empty());
    }

    #[test]
    fn second_forget_returns_zero() {
        let (idx, _tmp) = make_index();
        let ts = PrincipalNamespaceIndex::now_ns();

        idx.record_write("alice@example.org", 10, "calendar", "e1", ts)
            .unwrap();
        let d1 = idx.forget("alice@example.org").unwrap();
        assert_eq!(d1, 1);
        let d2 = idx.forget("alice@example.org").unwrap();
        assert_eq!(d2, 0);
    }
}
