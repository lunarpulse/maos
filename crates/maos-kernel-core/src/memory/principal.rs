#![forbid(unsafe_code)]

//! Principal Namespace Index — kernel-side address-only index of
//! `principal:<principal_id>:<schema>` writes per ADR-026 (Story 4.3).
//!
//! The index makes subject-access query O(log N) on `principal_id`
//! without scanning the private store.  Carries NO content — only the
//! addressing tuple `(principal_id, writer_owner, schema, key)` plus the audit pid.

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
    writer_owner TEXT NOT NULL,
    PRIMARY KEY (principal_id, writer_owner, schema, key)
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
        conn.execute_batch("BEGIN IMMEDIATE;")
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        conn.execute_batch(CREATE_TABLE_SQL)
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        // Existing databases first gain an owner column, then replace the pid key.
        let has_owner: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('principal_index') WHERE name='writer_owner')",
                [],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        if !has_owner {
            conn.execute_batch(
                "ALTER TABLE principal_index ADD COLUMN writer_owner TEXT NOT NULL DEFAULT ''",
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        }
        let pid_key: bool = conn
            .query_row(
                "SELECT pk != 0 FROM pragma_table_info('principal_index') WHERE name='writer_spirit_pid'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        if pid_key {
            conn.execute_batch(&format!(
                "ALTER TABLE principal_index RENAME TO principal_index_legacy;
                 DROP INDEX principal_index_id_idx;
                 {CREATE_TABLE_SQL}
                 INSERT INTO principal_index
                 SELECT principal_id, writer_spirit_pid, schema, key, timestamp_ns,
                        CASE WHEN writer_owner = '' THEN CAST(writer_spirit_pid AS TEXT) ELSE writer_owner END
                 FROM principal_index_legacy;
                 DROP TABLE principal_index_legacy;"
            ))
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        }
        conn.execute_batch("COMMIT;")
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ------------------------------------------------------------------
    // public API
    // ------------------------------------------------------------------

    /// Record a principal-namespace write — called by `MemoryManagerAdapter::write`
    /// whenever `namespace == MemoryNamespace::Principal { .. }`.
    pub(in crate::memory) fn record_write(
        &self,
        principal_id: &str,
        writer_spirit_pid: u32,
        writer_owner: &str,
        schema: &str,
        key: &str,
        timestamp_ns: u64,
        spill: impl FnOnce() -> Result<(), MemoryError>,
    ) -> Result<(), MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");
        let inserted = conn
            .execute(
                "INSERT OR IGNORE INTO principal_index
                (principal_id, writer_spirit_pid, schema, key, timestamp_ns, writer_owner)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    principal_id,
                    writer_spirit_pid as i64,
                    schema,
                    key,
                    timestamp_ns as i64,
                    writer_owner,
                ],
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        // Durable intent precedes the spill: a crash may leave redacted-missing,
        // never an undiscoverable value. Keep pre-existing rows on spill failure.
        if let Err(error) = spill() {
            if inserted != 0 {
                conn.execute("DELETE FROM principal_index WHERE principal_id=?1 AND writer_owner=?2 AND schema=?3 AND key=?4",
                    params![principal_id, writer_owner, schema, key])
                    .map_err(|cleanup| MemoryError::Storage(format!("{error}; intent cleanup: {cleanup}")))?;
            }
            return Err(error);
        }
        conn.execute("UPDATE principal_index SET writer_spirit_pid=?1, timestamp_ns=?2 WHERE principal_id=?3 AND writer_owner=?4 AND schema=?5 AND key=?6",
            params![writer_spirit_pid, timestamp_ns as i64, principal_id, writer_owner, schema, key])
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        Ok(())
    }

    /// Subject-access query — stable owner addresses plus boot-local audit pids
    /// for the given `principal_id` across ALL Spirits on this Host.
    /// Sorted by `(writer_spirit_pid, writer_owner, schema, key)`.
    pub(in crate::memory) fn lookup(
        &self,
        principal_id: &str,
    ) -> Result<Vec<PrincipalIndexRow>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");

        let mut stmt = conn
            .prepare(
                "SELECT principal_id, writer_spirit_pid, schema, key, timestamp_ns, writer_owner
                 FROM principal_index
                 WHERE principal_id = ?1
                 ORDER BY writer_spirit_pid, writer_owner, schema, key",
            )
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        let rows = stmt
            .query_map(params![principal_id], |row| {
                Ok(PrincipalIndexRow {
                    principal_id: row.get(0)?,
                    writer_spirit_pid: row.get(1)?,
                    schema: row.get(2)?,
                    key: row.get(3)?,
                    timestamp_ns: row.get::<_, i64>(4)? as u64,
                    writer_owner: row.get(5)?,
                })
            })
            .map_err(|e| MemoryError::Storage(e.to_string()))?;

        rows.collect::<Result<_, _>>()
            .map_err(|e| MemoryError::Storage(e.to_string()))
    }

    pub(in crate::memory) fn principal_ids_for_owner(
        &self,
        owner: &str,
    ) -> Result<Vec<String>, MemoryError> {
        let conn = self
            .conn
            .lock()
            .expect("PrincipalNamespaceIndex lock poisoned");
        let mut stmt = conn.prepare("SELECT DISTINCT principal_id FROM principal_index WHERE writer_owner=?1 ORDER BY principal_id")
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        let rows = stmt
            .query_map([owner], |row| row.get(0))
            .map_err(|e| MemoryError::Storage(e.to_string()))?;
        rows.collect::<Result<_, _>>()
            .map_err(|e| MemoryError::Storage(e.to_string()))
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
