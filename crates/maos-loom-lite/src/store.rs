#![forbid(unsafe_code)]

//! Postgres+pgvector backing store for the Loom-lite collective tier.
//!
//! All database operations are async (tokio-postgres).  The sync
//! `CollectiveMemoryPort` adapter in `adapter.rs` crosses the boundary
//! via `spawn_blocking` + an injected runtime handle.

use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue};
use std::time::Duration;
use tokio_postgres::NoTls;

use crate::schema;

/// Configuration for the Loom-lite Postgres store.
pub struct StoreConfig {
    /// Postgres connection string (e.g. "host=localhost dbname=loom_lite").
    pub connection_string: String,
    /// Vector dimension for pgvector column.
    pub vector_dim: usize,
    /// Connection pool size (applied to the deadpool `PoolConfig`).
    pub pool_size: usize,
    /// Operation timeout in milliseconds.
    pub timeout_ms: u64,
    /// Home region (canonical ascii-v1) for CRDT source_region stamping.
    /// Empty string = no region configured (pre-11.2a behavior / single-region).
    pub home_region: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            connection_string: "host=localhost dbname=loom_lite".to_string(),
            vector_dim: schema::DEFAULT_VECTOR_DIM,
            pool_size: 16,
            timeout_ms: 5000,
            home_region: String::new(),
        }
    }
}

/// Async Postgres+pgvector backing store.
pub struct LoomLiteStore {
    pool: Pool,
    config: StoreConfig,
}

/// Store-level error type.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("postgres pool error: {0}")]
    Pool(String),
    #[error("postgres query error: {0}")]
    Query(String),
    #[error("schema error: {0}")]
    Schema(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("operation timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
}

impl From<tokio_postgres::Error> for StoreError {
    fn from(e: tokio_postgres::Error) -> Self {
        StoreError::Query(e.to_string())
    }
}

impl From<deadpool_postgres::PoolError> for StoreError {
    fn from(e: deadpool_postgres::PoolError) -> Self {
        StoreError::Pool(e.to_string())
    }
}

impl LoomLiteStore {
    /// Create a new store with the given config.
    ///
    /// `sslmode`: v1.5 ships NoTls-only.  An operator EXPLICITLY requesting
    /// `sslmode=require`/`verify-ca`/`verify-full`/`prefer` gets a hard ERROR
    /// (refusing to silently send plaintext credentials over an unencrypted
    /// link) rather than a silent downgrade.  `sslmode=disable`/unset is
    /// accepted.  `connect_timeout`/`keepalives`/`keepalives_idle` are
    /// forwarded (no silent drop).  `pool_size` is applied to the deadpool.
    pub async fn new(config: StoreConfig) -> Result<Self, StoreError> {
        // sslmode guard — detected at the string level because
        // tokio_postgres::Config::get_ssl_mode returns a non-Option default
        // that cannot distinguish "unset" from "explicitly set".
        let conn_lower = config.connection_string.to_lowercase();
        let explicit_tls = conn_lower.split_whitespace().any(|kv| {
            kv.strip_prefix("sslmode=")
                .is_some_and(|v| matches!(v, "require" | "verify-ca" | "verify-full" | "prefer"))
        });
        if explicit_tls {
            return Err(StoreError::Schema(
                "sslmode=require/verify-ca/verify-full/prefer requested in the connection \
                 string, but maos-loom-lite v1.5 ships NoTls-only. Set sslmode=disable \
                 (cleartext, e.g. loopback) or front the service with a TLS-terminating \
                 sidecar. Refusing to silently send plaintext credentials over an \
                 unencrypted link."
                    .to_string(),
            ));
        }

        let mut pg_config = Config::new();
        let pg_parsed: tokio_postgres::Config = config
            .connection_string
            .parse()
            .map_err(|e: tokio_postgres::Error| StoreError::Schema(e.to_string()))?;

        if let Some(hosts) = pg_parsed.get_hosts().first() {
            match hosts {
                tokio_postgres::config::Host::Tcp(h) => pg_config.host = Some(h.clone()),
                #[cfg(unix)]
                tokio_postgres::config::Host::Unix(p) => {
                    pg_config.host = Some(p.to_string_lossy().into_owned())
                }
            }
        }
        if let Some(ports) = pg_parsed.get_ports().first() {
            pg_config.port = Some(*ports);
        }
        if let Some(user) = pg_parsed.get_user() {
            pg_config.user = Some(user.to_string());
        }
        if let Some(dbname) = pg_parsed.get_dbname() {
            pg_config.dbname = Some(dbname.to_string());
        }
        if let Some(password) = pg_parsed.get_password() {
            pg_config.password = Some(String::from_utf8_lossy(password).into_owned());
        }
        // Forward connection-level timeouts/keepalives (no silent drop).
        pg_config.connect_timeout = pg_parsed.get_connect_timeout().copied();
        pg_config.keepalives = Some(pg_parsed.get_keepalives());
        pg_config.keepalives_idle = Some(pg_parsed.get_keepalives_idle());

        pg_config.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        // Apply pool_size (was a dead field — AC1 review).
        pg_config.pool = Some(PoolConfig {
            max_size: config.pool_size,
            ..Default::default()
        });

        let pool = pg_config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| StoreError::Pool(e.to_string()))?;

        Ok(Self { pool, config })
    }

    /// Initialize the schema (idempotent).
    pub async fn init_schema(&self) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        let sql = schema::create_schema_sql(self.config.vector_dim);
        client
            .batch_execute(&sql)
            .await
            .map_err(|e| StoreError::Schema(e.to_string()))?;

        // Set HNSW session params on this pooled connection.
        client
            .batch_execute(schema::HNSW_SESSION_INIT)
            .await
            .map_err(|e| StoreError::Schema(e.to_string()))?;

        Ok(())
    }

    /// Write a value to the collective store (upsert with CRDT LWW merge).
    ///
    /// Story 11.2a (AC1): local writes stamp `source_ts = now()` and
    /// `source_region = config.home_region`.  The upsert is **conditional**:
    /// overwrites ONLY when the incoming `(source_ts, source_region)` total
    /// order strictly dominates the stored one (LWW-register property).
    pub async fn write(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
    ) -> Result<(), StoreError> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as i64;
        self.write_with_source(
            spirit_pid,
            namespace,
            key,
            value,
            ts,
            &self.config.home_region,
            "",
        )
        .await
    }

    /// Write with explicit source provenance (cross-region replication path).
    ///
    /// Story 11.2a (AC1/D1): the CRDT LWW merge — conditional upsert that
    /// overwrites ONLY when the incoming `(source_ts, source_region)` total
    /// order strictly dominates the stored one.  Total order is lexicographic:
    /// `source_ts` first (higher wins), then `source_region` (lexicographic
    /// tiebreak on identical timestamps).  This is commutative + idempotent:
    /// the converged value is identical regardless of arrival order.
    ///
    /// `source_ts` is the **source write's** nanosecond timestamp, preserved
    /// across re-attestation apply — NOT re-minted on apply.
    pub async fn write_with_source(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        source_ts: i64,
        source_region: &str,
        source_log_ref: &str,
    ) -> Result<(), StoreError> {
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);
        let (val_kind, val_data) =
            schema::value_to_parts(&value).map_err(StoreError::Serialization)?;

        // Story 11.2a (AC1): conditional LWW upsert.
        // Total order: (source_ts, source_region) lexicographic.
        // Overwrite ONLY when incoming strictly dominates stored:
        //   incoming.source_ts > stored.source_ts
        //   OR (incoming.source_ts == stored.source_ts AND incoming.source_region > stored.source_region)
        // This replaces the unconditional DO UPDATE (store.rs:188-191).
        client
            .execute(
                "INSERT INTO collective_memory
                    (spirit_pid, namespace_kind, namespace_detail, key,
                     value_kind, value_data, timestamp_ns, source_ts, source_region, source_log_ref)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                 ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                 DO UPDATE SET
                     value_kind = EXCLUDED.value_kind,
                     value_data = EXCLUDED.value_data,
                     timestamp_ns = EXCLUDED.timestamp_ns,
                     source_ts = EXCLUDED.source_ts,
                     source_region = EXCLUDED.source_region,
                     source_log_ref = EXCLUDED.source_log_ref
                 WHERE EXCLUDED.source_ts > collective_memory.source_ts
                    OR (EXCLUDED.source_ts = collective_memory.source_ts
                        AND EXCLUDED.source_region > collective_memory.source_region)",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &key,
                    &val_kind,
                    &val_data,
                    &source_ts,
                    &source_ts,
                    &source_region,
                    &source_log_ref,
                ],
            )
            .await?;

        Ok(())
    }

    /// Read a value from the collective store.
    pub async fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, StoreError> {
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);

        let rows = client
            .query(
                "SELECT value_kind, value_data FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2 AND namespace_detail = $3 AND key = $4",
                &[&(spirit_pid as i64), &ns_kind, &ns_detail, &key],
            )
            .await?;

        if let Some(row) = rows.first() {
            let val_kind: &str = row.get(0);
            let val_data: Vec<u8> = row.get(1);
            let value =
                schema::parts_to_value(val_kind, &val_data).map_err(StoreError::Serialization)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Scan entries matching a key prefix.
    ///
    /// An entry that fails validation is PROPAGATED as an error (AC1 review —
    /// no silent truncation of results).
    pub async fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);
        let escaped_prefix = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like_pattern = format!("{escaped_prefix}%");

        let rows = client
            .query(
                "SELECT namespace_kind, namespace_detail, key, value_kind, value_data, timestamp_ns
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2 AND namespace_detail = $3 AND key LIKE $4
                 ORDER BY key ASC
                 LIMIT $5",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &like_pattern,
                    &(limit as i64),
                ],
            )
            .await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in &rows {
            let row_ns_kind: &str = row.get(0);
            let row_ns_detail: String = row.get(1);
            let row_key: String = row.get(2);
            let row_val_kind: &str = row.get(3);
            let row_val_data: Vec<u8> = row.get(4);
            let row_ts: i64 = row.get(5);

            let ns = schema::parts_to_namespace(row_ns_kind, &row_ns_detail)
                .map_err(StoreError::Serialization)?;
            let val = schema::parts_to_value(row_val_kind, &row_val_data)
                .map_err(StoreError::Serialization)?;

            let entry = MemoryEntry::new(ns, row_key, val, row_ts as u64)
                .map_err(|e| StoreError::Serialization(format!("invalid entry: {e}")))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Read all collective-memory rows with source provenance for the
    /// convergence oracle (Story 11.2a AC3).
    ///
    /// Generic over `GenericClient` so it works with both `Client` and
    /// `Transaction` (verify-before-commit pattern from `canonical.rs:281`).
    pub async fn read_all_rows_from<C>(
        client: &C,
    ) -> Result<Vec<CollectiveRow>, StoreError>
    where
        C: tokio_postgres::GenericClient + Sync,
    {
        let rows = client
            .query(
                "SELECT spirit_pid, namespace_kind, namespace_detail, key,
                        value_kind, value_data, source_region, source_ts, source_log_ref
                 FROM collective_memory
                 ORDER BY spirit_pid, namespace_kind, namespace_detail, key",
                &[],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;

        Ok(rows
            .iter()
            .map(|row| CollectiveRow {
                spirit_pid: row.get(0),
                namespace_kind: row.get(1),
                namespace_detail: row.get(2),
                key: row.get(3),
                value_kind: row.get(4),
                value_data: row.get(5),
                source_region: row.get(6),
                source_ts: row.get(7),
                source_log_ref: row.get(8),
            })
            .collect())
    }

    /// Get a client with the configured timeout.
    async fn get_client_with_timeout(&self) -> Result<deadpool_postgres::Client, StoreError> {
        let timeout = Duration::from_millis(self.config.timeout_ms);
        match tokio::time::timeout(timeout, self.pool.get()).await {
            Ok(Ok(client)) => Ok(client),
            Ok(Err(e)) => Err(StoreError::Pool(e.to_string())),
            Err(_) => Err(StoreError::Timeout {
                timeout_ms: self.config.timeout_ms,
            }),
        }
    }

    /// Access the pool for migration and other low-level ops.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Access the store config.
    pub fn config(&self) -> &StoreConfig {
        &self.config
    }
}

/// A raw collective-memory row for convergence oracle + replication.
///
/// Story 11.2a (AC3): the oracle triple needs the full row with source
/// provenance fields.  This struct is the internal shape — NOT exported
/// through the `CollectiveMemoryPort` trait.
#[derive(Debug, Clone)]
pub struct CollectiveRow {
    pub spirit_pid: i64,
    pub namespace_kind: String,
    pub namespace_detail: String,
    pub key: String,
    pub value_kind: String,
    pub value_data: Vec<u8>,
    pub source_region: String,
    pub source_ts: i64,
    pub source_log_ref: String,
}

impl From<StoreError> for MemoryError {
    fn from(e: StoreError) -> Self {
        MemoryError::Storage(e.to_string())
    }
}
