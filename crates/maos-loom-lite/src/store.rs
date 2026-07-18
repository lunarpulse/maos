#![forbid(unsafe_code)]

//! Postgres+pgvector backing store for the Loom-lite collective tier.
//!
//! All database operations are async (tokio-postgres).  The sync
//! `CollectiveMemoryPort` adapter in `adapter.rs` crosses the boundary
//! via `spawn_blocking` + an injected runtime handle.

use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue};
use std::sync::Arc;
use std::time::Duration;
use tokio_postgres::NoTls;

use crate::schema;
use crate::seal::{AtRestSeal, AtRestSealer};
use crate::tenant::{TenantMapError, TenantMapPort};
use maos_domain::team::TeamId;

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
    /// Home team for physical tenant isolation.
    /// Empty string = tenancy disabled.
    pub home_team: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            connection_string: "host=localhost dbname=loom_lite".to_string(),
            vector_dim: schema::DEFAULT_VECTOR_DIM,
            pool_size: 16,
            timeout_ms: 5000,
            home_region: String::new(),
            home_team: String::new(),
        }
    }
}

/// Async Postgres+pgvector backing store.
pub struct LoomLiteStore {
    pool: Pool,
    config: StoreConfig,
    /// Story 11.4c (AC3): optional at-rest seal hook applied to value
    /// payload bytes at the write layer. `None` (default) = byte-identical
    /// Option-A plaintext.
    at_rest_sealer: AtRestSealer,
    tenant_map: Option<Arc<dyn TenantMapPort>>,
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
    /// Story 11.4c (AC3): the configured at-rest seal hook failed. The write
    /// fails CLOSED — no plaintext is persisted under a configured seal
    /// posture.
    #[error("at-rest seal error: {0}")]
    AtRestSeal(String),
    #[error("tenant map stale: {0}")]
    TenantMapStale(String),
    #[error("tenant connection mismatch: {0}")]
    TenantConnectionMismatch(String),
    #[error("tenant spirit pid {0} is not registered")]
    TenantSpiritUnmapped(u32),
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

        Ok(Self {
            pool,
            config,
            at_rest_sealer: AtRestSealer::default(),
            tenant_map: None,
        })
    }

    /// Story 11.4c (AC3): install an optional at-rest seal hook.
    ///
    /// When `Some`, value payload bytes are sealed (ciphertext) at the write
    /// layer before persistence; when `None`, writes are byte-identical
    /// Option-A plaintext (the default). On hook error the write fails
    /// CLOSED — never persist plaintext under a configured seal posture.
    ///
    /// The hook is a **closure boundary**: `maos-loom-lite` carries NO
    /// dependency on `maos-secrets` (L7 closure hygiene). The daemon
    /// composition root builds the closure from
    /// `maos_secrets::seal_at_rest_opt` bound to the configured
    /// `KeyManagementPort` + `CryptoProvider` and injects it here.
    pub fn with_at_rest_seal(mut self, seal: Option<AtRestSeal>) -> Self {
        self.at_rest_sealer = AtRestSealer::new(seal);
        self
    }

    /// Install the verified tenant-map lookup owned by the composition root.
    pub fn with_tenant_map(mut self, tenant_map: Arc<dyn TenantMapPort>) -> Self {
        self.tenant_map = Some(tenant_map);
        self
    }

    /// Story 11.4c (AC3): the configured at-rest sealer (introspectable so
    /// the composition root / operators can confirm seal posture).
    pub fn at_rest_sealer(&self) -> &AtRestSealer {
        &self.at_rest_sealer
    }

    /// Initialize the schema (idempotent).
    pub async fn init_schema(&self) -> Result<(), StoreError> {
        let client = self.pool.get().await?;
        if !self.config.home_team.is_empty() {
            let row = client
                .query_one("SELECT current_database()", &[])
                .await
                .map_err(|error| StoreError::Query(error.to_string()))?;
            let current_database: String = row.get(0);
            self.connection_assignment_guard(&current_database)?;
        }
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
        self.team_guard(spirit_pid)?;
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
    ///
    /// Deliberately unguarded: this is the verified replication apply path,
    /// not a Spirit-facing entry point, and this store can name only its own
    /// configured Postgres database.
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

        // Story 11.4c (AC3): seal the value payload at the write layer. With
        // a configured hook, `val_data` is transformed into ciphertext BEFORE
        // the INSERT (sealed-at-rest); without one (default) it is
        // byte-identical Option-A plaintext. On hook error the write fails
        // CLOSED here — the `?` returns before `execute`, so NO plaintext is
        // persisted under a configured seal posture.
        let val_data = self.at_rest_sealer.seal(&val_data)?;

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
    ///
    /// Story 11.2b (AC4 / F4): enforces fail-closed region-identity on the LIVE
    /// read path via [`region_guard`]. The store now SELECTs the row's
    /// `source_region` + `source_log_ref` (store-internal provenance) but does
    /// NOT return them — `CollectiveMemoryPort::read` stays byte-identical. An
    /// un-validated foreign-region row is refused (returns `None`), never served.
    pub async fn read(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<MemoryValue>, StoreError> {
        self.team_guard(spirit_pid)?;
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);

        let rows = client
            .query(
                "SELECT value_kind, value_data, source_region, source_log_ref
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2 AND namespace_detail = $3 AND key = $4",
                &[&(spirit_pid as i64), &ns_kind, &ns_detail, &key],
            )
            .await?;

        if let Some(row) = rows.first() {
            let val_kind: &str = row.get(0);
            let val_data: Vec<u8> = row.get(1);
            let src_region: String = row.get(2);
            let src_log_ref: String = row.get(3);
            // AC4 (F4): fail-closed region-identity — refuse an un-validated
            // foreign-region row (NFR-Comp-4 "no transparent replication").
            if !self.region_guard(&src_region, &src_log_ref) {
                return Ok(None);
            }
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
    /// no silent truncation of results). Story 11.2b (AC4 / F4): enforces
    /// fail-closed region-identity — an un-validated foreign-region row is
    /// FILTERED OUT (never served), not errored; the SQL WHERE clause excludes
    /// foreign rows lacking a mediator stamp BEFORE the LIMIT is applied (so
    /// foreign rows do not consume LIMIT slots), and the Rust-side
    /// `region_guard` check is kept as defense-in-depth.
    pub async fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        self.team_guard(spirit_pid)?;
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);
        let escaped_prefix = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like_pattern = format!("{escaped_prefix}%");

        let rows = client
            .query(
                "SELECT namespace_kind, namespace_detail, key, value_kind, value_data, timestamp_ns, source_region, source_log_ref
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2 AND namespace_detail = $3 AND key LIKE $4
                   AND (source_region = $6 OR source_log_ref <> '')
                 ORDER BY key ASC
                 LIMIT $5",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &like_pattern,
                    &(limit as i64),
                    &self.config.home_region.as_str(),
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
            let src_region: String = row.get(6);
            let src_log_ref: String = row.get(7);

            // AC4 (F4): fail-closed region-identity — filter out an un-validated
            // foreign-region row (NFR-Comp-4 "no transparent replication").
            if !self.region_guard(&src_region, &src_log_ref) {
                continue;
            }

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

    /// Construction-time physical assignment check.
    ///
    /// Row ownership is physical: a row belongs to the team whose configured
    /// database contains it. No source-team column or row-level team predicate
    /// exists in Story 13.1. `init_schema` invokes this exactly once before
    /// schema work; Spirit reads never repeat the database-name query.
    fn connection_assignment_guard(&self, current_database: &str) -> Result<(), StoreError> {
        if self.config.home_team.is_empty() {
            return Ok(());
        }
        let home_team = TeamId::new(&self.config.home_team)
            .map_err(|error| StoreError::TenantMapStale(error.to_string()))?;
        let tenant_map = self.tenant_map.as_ref().ok_or_else(|| {
            StoreError::TenantMapStale("tenant map is not configured".to_string())
        })?;
        let expected_database = tenant_map
            .datname_for(&home_team)
            .map_err(|error| StoreError::TenantMapStale(error.to_string()))?;
        if expected_database != current_database {
            return Err(StoreError::TenantConnectionMismatch(format!(
                "team {home_team} expects database {expected_database}, connected to {current_database}"
            )));
        }
        Ok(())
    }

    /// Story 13.1 (AC1): fail-closed physical-tenant guard for every
    /// Spirit-facing read, scan, and write.
    ///
    /// The guard runs once per call, before any query. Tenancy disabled
    /// (`home_team` empty) preserves the existing single-tenant behavior.
    ///
    /// # Trust boundary
    ///
    /// This stage proves manifest-membership presence only. It does not prove
    /// the per-team cryptographic key boundary owned by Story 13.2; a forged
    /// same-region team stamp remains a documented D1 exposure. Both operands
    /// remain store-internal (manifest-resolved caller team versus
    /// `config.home_team`), so `CollectiveMemoryPort` stays byte-identical.
    fn team_guard(&self, spirit_pid: u32) -> Result<(), StoreError> {
        if self.config.home_team.is_empty() {
            return Ok(());
        }
        let tenant_map = self.tenant_map.as_ref().ok_or_else(|| {
            StoreError::TenantMapStale("tenant map is not configured".to_string())
        })?;
        let caller_team = tenant_map
            .team_of(spirit_pid)
            .map_err(|error| match error {
                TenantMapError::Stale { reason } | TenantMapError::StateUnavailable { reason } => {
                    StoreError::TenantMapStale(reason)
                }
                TenantMapError::SpiritUnmapped { spirit_pid } => {
                    StoreError::TenantSpiritUnmapped(spirit_pid)
                }
                TenantMapError::TeamUnknown { team_id } => StoreError::TenantMapStale(format!(
                    "team {team_id} is absent from the verified tenant map"
                )),
            })?;
        if caller_team.as_str() != self.config.home_team {
            return Err(StoreError::TenantConnectionMismatch(format!(
                "store team {}, caller team {}",
                self.config.home_team, caller_team
            )));
        }
        Ok(())
    }

    /// Story 11.2b (AC4 / F4): fail-closed region-identity guard for the
    /// Spirit-facing READ path.
    ///
    /// Returns `true` (serve) iff the row is **home-origin** (`source_region`
    /// == `self.config.home_region`) OR a **foreign row carrying a
    /// provenance-presence marker** (a non-empty `source_log_ref` — the stamp
    /// `apply_replication_bundle` writes after `verify_replication_bundle`
    /// succeeds).
    ///
    /// # Trust boundary
    ///
    /// Cryptographic validation happens at **apply-time** (the Ed25519 bundle
    /// signature in `verify_replication_bundle`), NOT on every read.  The
    /// read-path guard checks **provenance-presence** (non-empty mediator
    /// stamp), not cryptographic validity.  A row whose `source_log_ref` is
    /// non-empty but *forged* (written via `write_with_source` or raw SQL
    /// without going through the signed mediator) **will be served** — this is
    /// a documented residual threat under the model where the attacker has
    /// direct DB write access (but that attacker can also INSERT a home-origin
    /// row the guard always serves, so forged-stamp is not a net-new
    /// exposure).  The successor that achieves per-row cryptographic validity
    /// is a **trusted-applied-root registry** (record verified roots at apply
    /// time, check on read) — a schema + apply-path feature out of 11.2b
    /// scope, tracked as a v2.x successor.
    ///
    /// Both operands are store-internal (the row's stored columns vs the
    /// store's `home_region` config) → `CollectiveMemoryPort::read/scan` stays
    /// byte-identical (ZERO kernel-Δ; the port gains no region parameter and
    /// the store does NOT return `source_region`). This is distinct from — and
    /// does NOT reuse — `DowngradeRouter::check_region_identity` (wrong home:
    /// it carries the *router's* home, not the *store's*; reusing it would
    /// couple the store to the router).
    ///
    /// NOTE on the F4 lean signature: the preflight prose named a 1-operand
    /// `region_guard(&self, row_source_region)`. AC4's binding text ("home-origin
    /// OR carry a validly-re-attested provenance") REQUIRES the `source_log_ref`
    /// operand to distinguish a validly-re-attested foreign row (served) from a
    /// raw-copy injection (refused). A 1-operand guard would either refuse ALL
    /// foreign rows (breaking 11.2a convergence — region-B could never read
    /// region-A's replicated data via `read`) or serve ALL foreign rows (no
    /// enforcement). The 2-operand form is the honest encoding of the property.
    fn region_guard(&self, row_source_region: &str, row_source_log_ref: &str) -> bool {
        if row_source_region == self.config.home_region {
            return true; // home-origin: always served
        }
        // Foreign row: serve ONLY with a provenance-presence marker — the
        // non-empty source_log_ref stamped by apply_replication_bundle after
        // the bundle signature was verified.  This is a PRESENCE check, not
        // cryptographic re-validation on read (see trust-boundary doc above).
        !row_source_log_ref.is_empty()
    }

    /// Read all collective-memory rows with source provenance for the
    /// convergence oracle (Story 11.2a AC3).
    ///
    /// Generic over `GenericClient` so it works with both `Client` and
    /// `Transaction` (verify-before-commit pattern from `canonical.rs:281`).
    pub async fn read_all_rows_from<C>(client: &C) -> Result<Vec<CollectiveRow>, StoreError>
    where
        C: tokio_postgres::GenericClient + Sync,
    {
        let rows = client
            .query(
                "SELECT spirit_pid, namespace_kind, namespace_detail, key,
                        value_kind, value_data, source_region, source_ts, source_log_ref,
                        source_team
                 FROM collective_memory
                 ORDER BY spirit_pid, namespace_kind, namespace_detail, key",
                &[],
            )
            .await
            .map_err(|e| StoreError::Query(e.to_string()))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            // Story 13.2 (AC2): source_team is a nullable column. NULL → v1 leaf
            // (first-party local row); a value must be canonical (TeamId rejects
            // non-canonical) — a malformed stored tag is a data-integrity fault.
            let source_team_raw: Option<String> = row.get(9);
            let source_team = match source_team_raw {
                Some(raw) => Some(TeamId::new(&raw).map_err(|e| {
                    StoreError::Query(format!("invalid source_team column value: {e}"))
                })?),
                None => None,
            };
            out.push(CollectiveRow {
                spirit_pid: row.get(0),
                namespace_kind: row.get(1),
                namespace_detail: row.get(2),
                key: row.get(3),
                value_kind: row.get(4),
                value_data: row.get(5),
                source_region: row.get(6),
                source_ts: row.get(7),
                source_log_ref: row.get(8),
                source_team,
            });
        }
        Ok(out)
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
    ///
    /// Deliberately unguarded: callers use this for schema, migration, and
    /// physical-absence verification against this store's single database.
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
    /// Story 13.2 (AC2): source-team provenance. `None` (NULL column) for
    /// first-party local rows; `Some` only for re-attested cross-team copies
    /// (written by 13.3 — at 13.2 local writes leave this NULL).
    pub source_team: Option<TeamId>,
}

impl From<StoreError> for MemoryError {
    fn from(e: StoreError) -> Self {
        MemoryError::Storage(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seal::AtRestSeal;
    use maos_domain::ports::registry::SpiritId;
    use maos_domain::team::TeamId;
    use std::sync::Arc;

    struct StaticTenantMap;

    impl TenantMapPort for StaticTenantMap {
        fn team_of(&self, _spirit_pid: u32) -> Result<TeamId, TenantMapError> {
            TeamId::new("team-a").map_err(|error| TenantMapError::StateUnavailable {
                reason: error.to_string(),
            })
        }

        fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
            // Patch 3: branch on the team so a wrong-team operand (e.g. passing
            // home_region instead of home_team) is detectable, not masked by a
            // constant return.
            match team.as_str() {
                "team-a" => Ok("maos_team_a".to_string()),
                "team-b" => Ok("maos_team_b".to_string()),
                _ => Err(TenantMapError::TeamUnknown {
                    team_id: team.clone(),
                }),
            }
        }

        fn register_spirit(&self, _spirit_pid: u32, _spirit_id: SpiritId) {}
    }

    /// Stale-map stub: every lookup reports the lease expired.
    struct StaleTenantMap;

    impl TenantMapPort for StaleTenantMap {
        fn team_of(&self, _spirit_pid: u32) -> Result<TeamId, TenantMapError> {
            Err(TenantMapError::Stale {
                reason: "test: lease expired".to_string(),
            })
        }

        fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
            Err(TenantMapError::TeamUnknown {
                team_id: team.clone(),
            })
        }

        fn register_spirit(&self, _spirit_pid: u32, _spirit_id: SpiritId) {}
    }

    /// Unmapped-spirit stub: no spirit is bound to a team.
    struct UnmappedTenantMap;

    impl TenantMapPort for UnmappedTenantMap {
        fn team_of(&self, spirit_pid: u32) -> Result<TeamId, TenantMapError> {
            Err(TenantMapError::SpiritUnmapped { spirit_pid })
        }

        fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
            Err(TenantMapError::TeamUnknown {
                team_id: team.clone(),
            })
        }

        fn register_spirit(&self, _spirit_pid: u32, _spirit_id: SpiritId) {}
    }

    /// Patch 1 (F18): the store-level typed refusals for a STALE map and an
    /// UNMAPPED spirit MUST be produced by `team_guard` AT THE STORE — the
    /// mapping arms (store.rs match on `TenantMapError`) were previously proven
    /// only at the adapter/port layer, leaving `team_guard`'s translation
    /// behaviorally unverified. `team_guard` runs pre-query (before any pool
    /// access), so `read`/`write`/`scan` refuse without a live database.
    #[tokio::test]
    async fn team_guard_maps_stale_and_unmapped_refusals_at_the_store() {
        let dead = || StoreConfig {
            connection_string: "host=127.0.0.1 port=1 dbname=none connect_timeout=1".to_string(),
            timeout_ms: 500,
            home_team: "team-a".to_string(),
            ..StoreConfig::default()
        };

        let stale = LoomLiteStore::new(dead())
            .await
            .unwrap()
            .with_tenant_map(Arc::new(StaleTenantMap));
        assert!(matches!(
            stale.read(7, &MemoryNamespace::Default, "k").await,
            Err(StoreError::TenantMapStale(_))
        ));
        assert!(matches!(
            stale
                .write(
                    7,
                    &MemoryNamespace::Default,
                    "k",
                    MemoryValue::Text("v".to_string()),
                )
                .await,
            Err(StoreError::TenantMapStale(_))
        ));
        assert!(matches!(
            stale.scan(7, &MemoryNamespace::Default, "k", 8).await,
            Err(StoreError::TenantMapStale(_))
        ));

        let unmapped = LoomLiteStore::new(dead())
            .await
            .unwrap()
            .with_tenant_map(Arc::new(UnmappedTenantMap));
        assert!(matches!(
            unmapped.read(9, &MemoryNamespace::Default, "k").await,
            Err(StoreError::TenantSpiritUnmapped(9))
        ));
        assert!(matches!(
            unmapped
                .write(
                    9,
                    &MemoryNamespace::Default,
                    "k",
                    MemoryValue::Text("v".to_string()),
                )
                .await,
            Err(StoreError::TenantSpiritUnmapped(9))
        ));
    }

    /// Story 11.4c (AC3): the `.with_at_rest_seal` builder installs the seal
    /// hook on the store, and the default (`new`) carries NO hook — the
    /// byte-identical Option-A plaintext posture.
    ///
    /// `LoomLiteStore::new` succeeds against a dead host because the deadpool
    /// is lazy (no connection at construction — see `transport_failure.rs`),
    /// so the builder wiring is verifiable without a live Postgres. The seal
    /// TRANSFORM itself (the bytes that hit disk) is exercised in
    /// [`seal::tests`](crate::seal::tests).
    #[tokio::test]
    async fn with_at_rest_seal_builder_installs_hook_default_is_none() {
        let store = LoomLiteStore::new(StoreConfig {
            connection_string: "host=127.0.0.1 port=1 dbname=none connect_timeout=1".to_string(),
            timeout_ms: 500,
            ..StoreConfig::default()
        })
        .await
        .expect("lazy pool: store creation succeeds even with a dead host");

        // Default posture: no seal hook (byte-identical Option-A plaintext).
        assert!(
            !store.at_rest_sealer().is_configured(),
            "default store MUST have no seal hook (Option-A plaintext)"
        );

        // Install a deterministic XOR stand-in for AEAD (no maos-secrets dep).
        let xor_seal: AtRestSeal = Arc::new(|d: &[u8]| Ok(d.iter().map(|b| b ^ 0xA5).collect()));
        let sealed_store = store.with_at_rest_seal(Some(xor_seal));

        assert!(
            sealed_store.at_rest_sealer().is_configured(),
            ".with_at_rest_seal(Some) MUST install the hook"
        );

        // `.with_at_rest_seal(None)` is an explicit return to plaintext posture.
        let plain_store = sealed_store.with_at_rest_seal(None);
        assert!(
            !plain_store.at_rest_sealer().is_configured(),
            ".with_at_rest_seal(None) MUST clear the hook"
        );
    }

    #[tokio::test]
    async fn connection_assignment_guard_matches_manifest_datname() {
        let store = LoomLiteStore::new(StoreConfig {
            connection_string: "host=127.0.0.1 port=1 dbname=none connect_timeout=1".to_string(),
            timeout_ms: 500,
            home_team: "team-a".to_string(),
            ..StoreConfig::default()
        })
        .await
        .unwrap()
        .with_tenant_map(Arc::new(StaticTenantMap));

        store.connection_assignment_guard("maos_team_a").unwrap();
        assert!(matches!(
            store.connection_assignment_guard("maos_team_b"),
            Err(StoreError::TenantConnectionMismatch(_))
        ));
    }
}
