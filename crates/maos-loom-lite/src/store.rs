#![forbid(unsafe_code)]

//! Postgres+pgvector backing store for the Loom-lite collective tier.
//!
//! All database operations are async (tokio-postgres).  The sync
//! `CollectiveMemoryPort` adapter in `adapter.rs` crosses the boundary
//! via `spawn_blocking` + an injected runtime handle.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use deadpool_postgres::{Config, ManagerConfig, Pool, PoolConfig, RecyclingMethod, Runtime};
use maos_audit::erasure::merkle::{verify_proof, MerkleProof};
use maos_domain::invariants::i13::IntentLineage;
use maos_domain::memory::{MemoryEntry, MemoryError, MemoryNamespace, MemoryValue};
use maos_domain::ports::CollectiveEraseReceipt;
use maos_domain::region::Region;
use tokio_postgres::NoTls;

use crate::cross_team_consent::CrossTeamConsentPort;
use crate::replication::bundle::verify_team_root_signature;
use crate::replication::leaf::CollectiveKvLeaf;
use crate::schema;
use crate::seal::{AtRestSeal, AtRestSealer};
use crate::tenant::{TenantMapError, TenantMapPort};
use maos_domain::team::TeamId;

/// FNV-1a 64 of `b"maos-loom-lite-init-schema"`, represented as a PostgreSQL
/// signed `bigint` advisory-lock key.
const SCHEMA_INIT_ADVISORY_LOCK_KEY: i64 = -7_268_255_128_678_003_653;

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

/// Explicit source provenance for verified replication writes.
#[derive(Debug, Clone, Copy)]
pub struct WriteSource<'a> {
    pub ts: i64,
    pub region: &'a str,
    pub log_ref: &'a str,
    pub team: Option<&'a TeamId>,
    pub distillation_depth: Option<u32>,
    pub intent_lineage: Option<&'a IntentLineage>,
}

pub(crate) struct RowAttestation<'a> {
    pub leaf_canonical_hash: &'a [u8; 32],
    pub merkle_root: &'a [u8; 32],
    pub region_sig: &'a [u8],
    pub bundle_schema_version: u16,
    pub inclusion_proof: &'a MerkleProof,
}

fn cross_team_namespace_detail(source_team: &TeamId, original_detail: &str) -> String {
    format!(
        "xteam:{}:{}",
        source_team.as_str(),
        hex::encode(original_detail.as_bytes())
    )
}

/// Decode the logical namespace detail from the stable crossed-row marker.
pub(crate) fn original_cross_team_namespace_detail(
    source_team: &TeamId,
    stored_detail: &str,
) -> Option<String> {
    let prefix = format!("xteam:{}:", source_team.as_str());
    let encoded = stored_detail.strip_prefix(&prefix)?;
    let bytes = hex::decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}

/// The one-share identity and source generation pinned into a crossed row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossedRowOrigin {
    pub source_team: TeamId,
    pub emitter_host: String,
    pub op_id: String,
    pub source_ts: i64,
    pub source_region: String,
}

fn parse_cross_team_marker(stored_detail: &str) -> Option<(TeamId, String)> {
    let rest = stored_detail.strip_prefix("xteam:")?;
    let (team, encoded_and_metadata) = rest.split_once(':')?;
    let team = TeamId::new(team).ok()?;
    let encoded = encoded_and_metadata.split(':').next()?;
    let bytes = hex::decode(encoded).ok()?;
    let detail = String::from_utf8(bytes).ok()?;
    Some((team, detail))
}

fn reject_principal_namespace(namespace: &MemoryNamespace) -> Result<(), StoreError> {
    if let MemoryNamespace::Principal {
        principal_id,
        schema,
    } = namespace
    {
        return Err(StoreError::PrincipalNamespaceForbidden {
            principal_id: principal_id.clone(),
            schema: schema.clone(),
        });
    }
    Ok(())
}

fn erasure_lock_key(
    spirit_pid: u32,
    namespace_kind: &str,
    logical_namespace_detail: &str,
    key: &str,
) -> String {
    format!("{spirit_pid}\\0{namespace_kind}\\0{logical_namespace_detail}\\0{key}")
}

fn erasure_clock_dominates(
    erased_at_source_ts: i64,
    erased_at_source_region: &str,
    incoming_source_ts: i64,
    incoming_source_region: &str,
) -> bool {
    erased_at_source_ts > incoming_source_ts
        || (erased_at_source_ts == incoming_source_ts
            && erased_at_source_region >= incoming_source_region)
}

fn dominating_erasure_clock(
    row: Option<(i64, &str)>,
    now_ns: i64,
    fallback_region: &str,
) -> (i64, String) {
    match row {
        Some((source_ts, source_region)) if source_ts >= now_ns => {
            (source_ts, source_region.to_owned())
        }
        _ => (now_ns, fallback_region.to_owned()),
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
    cross_team_consent: Option<Arc<dyn CrossTeamConsentPort>>,
    team_verifying_keys: HashMap<(Region, TeamId), [u8; 32]>,
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
    #[error("tenant map stale: {reason}")]
    TenantMapStale {
        team_id: Option<TeamId>,
        reason: String,
    },
    #[error("tenant connection mismatch for store {configured_team}: {reason}")]
    TenantConnectionMismatch {
        configured_team: TeamId,
        caller_team: Option<TeamId>,
        reason: String,
    },
    #[error("tenant spirit pid {spirit_pid} is not registered")]
    TenantSpiritUnmapped { spirit_pid: u32 },
    #[error("cross-team consent denied: {from_team}->{to_team}, intent={intent}")]
    ConsentDenied {
        from_team: TeamId,
        to_team: TeamId,
        intent: String,
    },
    #[error("cross-team consent state stale: {reason}")]
    ConsentStateStale { reason: String },
    #[error("cross-team attestation invalid for {team_id}: {reason}")]
    AttestationInvalid { team_id: TeamId, reason: String },
    #[error("principal namespace is forbidden in collective storage: {principal_id}/{schema}")]
    PrincipalNamespaceForbidden {
        principal_id: String,
        schema: String,
    },
    #[error(
        "collective erasure tombstone dominates key {key}: erased at ({erased_at_source_ts}, {erased_at_source_region})"
    )]
    ErasureTombstoneDominates {
        key: String,
        erased_at_source_ts: i64,
        erased_at_source_region: String,
    },
    #[error("collective row generation changed before erase")]
    StaleGeneration,
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

fn decode_row_provenance(
    depth: i32,
    lineage_bytes: Option<&[u8]>,
    has_source_team: bool,
) -> Result<(Option<u32>, Option<IntentLineage>), StoreError> {
    match (depth, lineage_bytes) {
        (0, None) => Ok((None, None)),
        (value, Some(bytes)) if value > 0 => {
            // Provenance without a source team is a consistency violation —
            // the write side and the verifier both refuse the shape, so a
            // stored row like this never came through an admitted path.
            if !has_source_team {
                return Err(StoreError::Query(
                    "distillation_depth and intent_lineage require a source team".into(),
                ));
            }
            let depth = u32::try_from(value)
                .map_err(|_| StoreError::Query("invalid distillation_depth".into()))?;
            let lineage = serde_json::from_slice(bytes).map_err(|error| {
                StoreError::Query(format!("invalid intent_lineage column value: {error}"))
            })?;
            Ok((Some(depth), Some(lineage)))
        }
        _ => Err(StoreError::Query(
            "distillation_depth and intent_lineage must be present together".into(),
        )),
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
            cross_team_consent: None,
            team_verifying_keys: HashMap::new(),
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

    /// Install the verified directional consent decision owned by the
    /// composition root.
    pub fn with_cross_team_consent(
        mut self,
        cross_team_consent: Arc<dyn CrossTeamConsentPort>,
    ) -> Self {
        self.cross_team_consent = Some(cross_team_consent);
        self
    }

    pub(crate) fn cross_team_consent(&self) -> Option<&Arc<dyn CrossTeamConsentPort>> {
        self.cross_team_consent.as_ref()
    }

    /// Look up the composition-injected verifying key for a manifest-declared
    /// `(region, team)` pair. The apply path requires the claimed pair to be
    /// present (fail-closed) so a crossing never lands rows the read path
    /// could never verify (13.3 review, party-mode D1).
    pub(crate) fn team_verifying_key(&self, region: &Region, team: &TeamId) -> Option<&[u8; 32]> {
        self.team_verifying_keys
            .iter()
            .find_map(|((r, t), key)| (r == region && t == team).then_some(key))
    }

    /// Install public team keys derived at the composition root. The store
    /// never receives the root seed or any signing key.
    pub fn with_team_verifying_keys(
        mut self,
        team_verifying_keys: HashMap<(Region, TeamId), [u8; 32]>,
    ) -> Self {
        self.team_verifying_keys = team_verifying_keys;
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
        // Story 13.5g — the manifest-team authority runs first, and it MUST run
        // on the same pooled client the schema work below then uses: the guard's
        // whole claim is that *this connection* is assigned to the team's
        // database, and validating one session while mutating another would
        // break the entailment that lets Story 13.5g retire the Stage-2
        // reconcile (D-2). The pure query is shared with Phase B through
        // [`Self::current_database`], which takes its own client because the
        // composition root has no client to lend it.
        if !self.config.home_team.is_empty() {
            let current_database = Self::current_database_of(&client).await?;
            self.connection_assignment_guard(&current_database)?;
        }
        let client = self.pool.get().await?;
        let sql = schema::create_schema_sql(self.config.vector_dim);
        client
            .query_one(
                "SELECT pg_advisory_lock($1)",
                &[&SCHEMA_INIT_ADVISORY_LOCK_KEY],
            )
            .await
            .map_err(|error| StoreError::Schema(error.to_string()))?;
        let schema_result = client.batch_execute(&sql).await;
        let unlock_result = client
            .query_one(
                "SELECT pg_advisory_unlock($1)",
                &[&SCHEMA_INIT_ADVISORY_LOCK_KEY],
            )
            .await
            .map(|row| row.get::<_, bool>(0));
        match (schema_result, unlock_result) {
            (Err(error), _) => return Err(StoreError::Schema(error.to_string())),
            (Ok(()), Ok(true)) => {}
            (Ok(()), Ok(false)) => {
                return Err(StoreError::Schema(
                    "schema initialization advisory lock was not held".to_string(),
                ));
            }
            (Ok(()), Err(error)) => return Err(StoreError::Schema(error.to_string())),
        }

        // Set HNSW session params on this pooled connection.
        client
            .batch_execute(schema::HNSW_SESSION_INIT)
            .await
            .map_err(|e| StoreError::Schema(e.to_string()))?;

        Ok(())
    }

    /// The connected Postgres database name (Story 13.5g AC4 live operand).
    ///
    /// Exposes the same live value the `connection_assignment_guard` sees to the
    /// Phase B persisted-vs-live datname check in the composition root. Takes
    /// its own pooled client; callers that already hold one — [`Self::init_schema`]
    /// — use [`Self::current_database_of`] so the guard and the work it guards
    /// share a session. Returns `StoreError::Query` if the database is unavailable.
    pub async fn current_database(&self) -> Result<String, StoreError> {
        let client = self.pool.get().await?;
        Self::current_database_of(&client).await
    }

    /// `SELECT current_database()` on a caller-supplied client.
    async fn current_database_of(client: &deadpool_postgres::Client) -> Result<String, StoreError> {
        let row = client
            .query_one("SELECT current_database()", &[])
            .await
            .map_err(|error| StoreError::Query(error.to_string()))?;
        Ok(row.get(0))
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
            WriteSource {
                ts,
                region: &self.config.home_region,
                log_ref: "",
                team: None,
                distillation_depth: None,
                intent_lineage: None,
            },
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
        source: WriteSource<'_>,
    ) -> Result<(), StoreError> {
        self.write_with_source_attested(spirit_pid, namespace, key, value, source, None)
            .await
    }

    pub(crate) async fn write_with_source_attested(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        source: WriteSource<'_>,
        attestation: Option<&RowAttestation<'_>>,
    ) -> Result<(), StoreError> {
        self.write_with_source_attested_at(
            spirit_pid,
            namespace,
            key,
            value,
            source,
            attestation,
            true,
        )
        .await
    }

    /// Persist an attested first-party source row at its native address.
    pub(crate) async fn write_native_with_source_attested(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        source: WriteSource<'_>,
        attestation: Option<&RowAttestation<'_>>,
    ) -> Result<(), StoreError> {
        self.write_with_source_attested_at(
            spirit_pid,
            namespace,
            key,
            value,
            source,
            attestation,
            false,
        )
        .await
    }

    async fn write_with_source_attested_at(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        value: MemoryValue,
        source: WriteSource<'_>,
        attestation: Option<&RowAttestation<'_>>,
        crossed: bool,
    ) -> Result<(), StoreError> {
        reject_principal_namespace(namespace)?;
        // Provenance tuple validation (13.3b review): the row decoder accepts
        // only (depth 0, NULL lineage) or (depth >= 1, lineage); any other
        // combination stored here would fail every subsequent read of the key.
        // Mirrors CollectiveKvLeaf::has_valid_version_shape, including the
        // i32 representability bound and the non-empty lineage rule (the
        // writer rejects an empty computed lineage as AuditChainMissing).
        match (
            source.team,
            source.distillation_depth,
            source.intent_lineage,
        ) {
            (None, None, None) | (Some(_), None, None) => {}
            (Some(_), Some(depth), Some(lineage))
                if depth >= 1 && depth <= i32::MAX as u32 && !lineage.as_slice().is_empty() => {}
            _ => {
                return Err(StoreError::Serialization(
                    "invalid provenance shape: distillation_depth (1..=i32::MAX) and \
                     a non-empty intent_lineage must be present together, and \
                     provenance requires a source team"
                        .into(),
                ));
            }
        }
        let (ns_kind, logical_ns_detail) = schema::namespace_to_parts(namespace);
        let mut ns_detail = logical_ns_detail.clone();
        if crossed {
            if let Some(source_team) = source.team {
                ns_detail = cross_team_namespace_detail(source_team, &ns_detail);
            }
        }
        let (val_kind, val_data) =
            schema::value_to_parts(&value).map_err(StoreError::Serialization)?;
        let val_data = self.at_rest_sealer.seal(&val_data)?;

        let leaf_canonical_hash: Option<&[u8]> =
            attestation.map(|value| value.leaf_canonical_hash.as_slice());
        let merkle_root: Option<&[u8]> = attestation.map(|value| value.merkle_root.as_slice());
        let region_sig: Option<&[u8]> = attestation.map(|value| value.region_sig);
        let bundle_schema_version =
            attestation.map(|value| i16::try_from(value.bundle_schema_version).unwrap());
        let inclusion_path = attestation
            .map(|value| {
                serde_json::to_vec(value.inclusion_proof)
                    .map_err(|error| StoreError::Serialization(error.to_string()))
            })
            .transpose()?;
        let distillation_depth = source
            .distillation_depth
            .map(i32::try_from)
            .transpose()
            .map_err(|_| StoreError::Serialization("distillation_depth exceeds INTEGER".into()))?
            .unwrap_or(0);
        let intent_lineage = source
            .intent_lineage
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| StoreError::Serialization(error.to_string()))?;

        let mut client = self.get_client_with_timeout().await?;
        let transaction = client.transaction().await?;
        let lock_key = erasure_lock_key(spirit_pid, ns_kind, &logical_ns_detail, key);
        transaction
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lock_key],
            )
            .await?;
        if let Some(row) = transaction
            .query_opt(
                "SELECT erased_at_source_ts, erased_at_source_region
                 FROM collective_erasure_tombstones
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4",
                &[&(spirit_pid as i64), &ns_kind, &ns_detail, &key],
            )
            .await?
        {
            let erased_at_source_ts: i64 = row.get(0);
            let erased_at_source_region: String = row.get(1);
            if erasure_clock_dominates(
                erased_at_source_ts,
                &erased_at_source_region,
                source.ts,
                source.region,
            ) {
                return Err(StoreError::ErasureTombstoneDominates {
                    key: key.to_string(),
                    erased_at_source_ts,
                    erased_at_source_region,
                });
            }
        }

        let changed = transaction
            .execute(
                "INSERT INTO collective_memory
                    (spirit_pid, namespace_kind, namespace_detail, key,
                     value_kind, value_data, timestamp_ns, source_ts, source_region,
                     source_log_ref, source_team, distillation_depth, intent_lineage,
                     leaf_canonical_hash, merkle_root, region_sig,
                     bundle_schema_version, inclusion_path,
                     cross_emitter_host, cross_op_id, cross_source_ts, cross_source_region)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                         $13, $14, $15, $16, $17, $18, NULL, NULL, NULL, NULL)
                 ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                 DO UPDATE SET
                     value_kind = EXCLUDED.value_kind,
                     value_data = EXCLUDED.value_data,
                     timestamp_ns = EXCLUDED.timestamp_ns,
                     source_ts = EXCLUDED.source_ts,
                     source_region = EXCLUDED.source_region,
                     source_log_ref = EXCLUDED.source_log_ref,
                     source_team = EXCLUDED.source_team,
                     distillation_depth = EXCLUDED.distillation_depth,
                     intent_lineage = EXCLUDED.intent_lineage,
                     leaf_canonical_hash = EXCLUDED.leaf_canonical_hash,
                     merkle_root = EXCLUDED.merkle_root,
                     region_sig = EXCLUDED.region_sig,
                     bundle_schema_version = EXCLUDED.bundle_schema_version,
                     inclusion_path = EXCLUDED.inclusion_path,
                     cross_emitter_host = NULL,
                     cross_op_id = NULL,
                     cross_source_ts = NULL,
                     cross_source_region = NULL
                 WHERE ($19 OR EXCLUDED.source_team IS NULL
                        OR collective_memory.source_team IS NOT DISTINCT FROM EXCLUDED.source_team)
                   AND (
                       EXCLUDED.source_ts > collective_memory.source_ts
                       OR (EXCLUDED.source_ts = collective_memory.source_ts
                           AND EXCLUDED.source_region > collective_memory.source_region)
                   )",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &key,
                    &val_kind,
                    &val_data,
                    &source.ts,
                    &source.ts,
                    &source.region,
                    &source.log_ref,
                    &source.team.map(TeamId::as_str),
                    &distillation_depth,
                    &intent_lineage,
                    &leaf_canonical_hash,
                    &merkle_root,
                    &region_sig,
                    &bundle_schema_version,
                    &inclusion_path,
                    &(!crossed),
                ],
            )
            .await?;
        if !crossed && changed != 1 {
            return Err(StoreError::Serialization(
                "native originated row did not replace its existing generation".to_string(),
            ));
        }
        transaction.commit().await?;

        Ok(())
    }

    /// Resolve the complete one-share origin record for a crossed physical row.
    ///
    /// An unannotated crossed row is intentionally not reconciliation-eligible:
    /// it cannot bind an erase to one specific share operation.
    pub async fn crossed_row_origin(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<CrossedRowOrigin>, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let (namespace_kind, logical_namespace_detail) = schema::namespace_to_parts(namespace);
        let cross_team_pattern = format!(
            "xteam:%:{}",
            hex::encode(logical_namespace_detail.as_bytes())
        );
        let client = self.get_client_with_timeout().await?;
        let row = client
            .query_opt(
                "SELECT namespace_detail, source_team, cross_emitter_host, cross_op_id,
                        cross_source_ts, cross_source_region
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail LIKE $3 AND key = $4
                   AND source_team IS NOT NULL
                 ORDER BY source_ts DESC, source_region DESC
                 LIMIT 1",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &cross_team_pattern,
                    &key,
                ],
            )
            .await?;
        row.map(|row| {
            let detail: String = row.get(0);
            let persisted_team: String = row.get(1);
            let source_team = TeamId::new(&persisted_team).map_err(|error| {
                StoreError::Serialization(format!("invalid crossed-row source team: {error}"))
            })?;
            if detail != cross_team_namespace_detail(&source_team, &logical_namespace_detail) {
                return Err(StoreError::Serialization(
                    "crossed row marker disagrees with persisted source team".to_string(),
                ));
            }
            let (Some(emitter_host), Some(op_id), Some(source_ts), Some(source_region)) = (
                row.get::<_, Option<String>>(2),
                row.get::<_, Option<String>>(3),
                row.get::<_, Option<i64>>(4),
                row.get::<_, Option<String>>(5),
            ) else {
                return Err(StoreError::Serialization(
                    "crossed row is missing a valid share-operation binding".to_string(),
                ));
            };
            if op_id.len() != 32 || !op_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(StoreError::Serialization(
                    "crossed row has an invalid share-operation identifier".to_string(),
                ));
            }
            Ok(CrossedRowOrigin {
                source_team,
                emitter_host,
                op_id,
                source_ts,
                source_region,
            })
        })
        .transpose()
    }

    /// Attach a share operation's identity to the stable physical crossed row
    /// that just passed bundle verification. This is idempotent for a retry of
    /// the same share and never annotates a different generation.
    pub async fn annotate_crossed_row(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        origin: &CrossedRowOrigin,
    ) -> Result<(), StoreError> {
        reject_principal_namespace(namespace)?;
        let (namespace_kind, logical_namespace_detail) = schema::namespace_to_parts(namespace);
        let detail = cross_team_namespace_detail(&origin.source_team, &logical_namespace_detail);
        let client = self.get_client_with_timeout().await?;
        let changed = client
            .execute(
                "UPDATE collective_memory
                 SET cross_emitter_host = $5, cross_op_id = $6,
                     cross_source_ts = $7, cross_source_region = $8
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4
                   AND source_team = $9 AND source_ts = $7 AND source_region = $8",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &detail,
                    &key,
                    &origin.emitter_host,
                    &origin.op_id,
                    &origin.source_ts,
                    &origin.source_region,
                    &origin.source_team.as_str(),
                ],
            )
            .await?;
        if changed == 1 {
            Ok(())
        } else {
            Err(StoreError::Serialization(
                "crossed row changed before its share-operation binding was recorded".to_string(),
            ))
        }
    }

    /// Return the current native generation. `None` means that no native row
    /// remains (including the idempotent tombstoned state).
    pub async fn native_row_generation(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<Option<(i64, String)>, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let (namespace_kind, namespace_detail) = schema::namespace_to_parts(namespace);
        let client = self.get_client_with_timeout().await?;
        client
            .query_opt(
                "SELECT source_ts, source_region FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &namespace_detail,
                    &key,
                ],
            )
            .await
            .map(|row| row.map(|row| (row.get(0), row.get(1))))
            .map_err(StoreError::from)
    }

    /// Erase one store-addressed row and record a CRDT-LWW tombstone.
    ///
    /// Operator authority owns this path. The transaction-scoped advisory lock
    /// is shared with writes, preventing a stale replication apply from racing
    /// between the delete and tombstone insert.
    pub async fn erase(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
    ) -> Result<CollectiveEraseReceipt, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let (namespace_kind, logical_namespace_detail) = schema::namespace_to_parts(namespace);
        let cross_team_pattern = format!(
            "xteam:%:{}",
            hex::encode(logical_namespace_detail.as_bytes())
        );
        let lock_key = erasure_lock_key(spirit_pid, namespace_kind, &logical_namespace_detail, key);
        let mut client = self.get_client_with_timeout().await?;
        let transaction = client.transaction().await?;
        transaction
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lock_key],
            )
            .await?;
        let row = transaction
            .query_opt(
                "SELECT namespace_detail, source_ts, source_region
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND (namespace_detail = $3 OR namespace_detail LIKE $5)
                   AND key = $4
                 ORDER BY (namespace_detail = $3) DESC, source_ts DESC, source_region DESC
                 LIMIT 1
                 FOR UPDATE",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &logical_namespace_detail,
                    &key,
                    &cross_team_pattern,
                ],
            )
            .await?;
        let physical_namespace_detail = row
            .as_ref()
            .map(|row| row.get::<_, String>(0))
            .unwrap_or(logical_namespace_detail);
        let row_erasure_clock = row
            .as_ref()
            .map(|row| (row.get::<_, i64>(1), row.get::<_, String>(2)));
        let deleted_rows = transaction
            .execute(
                "DELETE FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &physical_namespace_detail,
                    &key,
                ],
            )
            .await?;
        let now_ns = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        )
        .unwrap_or(i64::MAX);
        let (erased_at_source_ts, erased_at_source_region) = dominating_erasure_clock(
            row_erasure_clock
                .as_ref()
                .map(|(source_ts, source_region)| (*source_ts, source_region.as_str())),
            now_ns,
            &self.config.home_region,
        );
        transaction
            .execute(
                "INSERT INTO collective_erasure_tombstones
                    (spirit_pid, namespace_kind, namespace_detail, key,
                     erased_at_source_ts, erased_at_source_region)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                 DO UPDATE SET
                     erased_at_source_ts = EXCLUDED.erased_at_source_ts,
                     erased_at_source_region = EXCLUDED.erased_at_source_region,
                     created_at = NOW()
                 WHERE EXCLUDED.erased_at_source_ts >
                           collective_erasure_tombstones.erased_at_source_ts
                    OR (EXCLUDED.erased_at_source_ts =
                           collective_erasure_tombstones.erased_at_source_ts
                        AND EXCLUDED.erased_at_source_region >
                           collective_erasure_tombstones.erased_at_source_region)",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &physical_namespace_detail,
                    &key,
                    &erased_at_source_ts,
                    &erased_at_source_region,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(CollectiveEraseReceipt {
            deleted_rows,
            tombstone_recorded: true,
        })
    }
    /// Erase the native row only if it remains the generation named by a
    /// verified reconciliation. The check and delete share the writer lock.
    pub async fn erase_at_generation(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        expected_source_ts: i64,
        expected_source_region: &str,
    ) -> Result<CollectiveEraseReceipt, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let (namespace_kind, namespace_detail) = schema::namespace_to_parts(namespace);
        let lock_key = erasure_lock_key(spirit_pid, namespace_kind, &namespace_detail, key);
        let mut client = self.get_client_with_timeout().await?;
        let transaction = client.transaction().await?;
        transaction
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lock_key],
            )
            .await?;
        let row = transaction
            .query_opt(
                "SELECT source_ts, source_region FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4
                 FOR UPDATE",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &namespace_detail,
                    &key,
                ],
            )
            .await?;
        if let Some(row) = row.as_ref() {
            if row.get::<_, i64>(0) != expected_source_ts
                || row.get::<_, String>(1) != expected_source_region
            {
                return Err(StoreError::StaleGeneration);
            }
        } else {
            let now_ns = i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            )
            .unwrap_or(i64::MAX);
            let (erased_at_source_ts, erased_at_source_region) = dominating_erasure_clock(
                Some((expected_source_ts, expected_source_region)),
                now_ns,
                &self.config.home_region,
            );
            transaction
                .execute(
                    "INSERT INTO collective_erasure_tombstones
                        (spirit_pid, namespace_kind, namespace_detail, key,
                         erased_at_source_ts, erased_at_source_region)
                     VALUES ($1, $2, $3, $4, $5, $6)
                     ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                     DO UPDATE SET
                         erased_at_source_ts = EXCLUDED.erased_at_source_ts,
                         erased_at_source_region = EXCLUDED.erased_at_source_region,
                         created_at = NOW()
                     WHERE EXCLUDED.erased_at_source_ts >
                               collective_erasure_tombstones.erased_at_source_ts
                        OR (EXCLUDED.erased_at_source_ts =
                               collective_erasure_tombstones.erased_at_source_ts
                            AND EXCLUDED.erased_at_source_region >
                               collective_erasure_tombstones.erased_at_source_region)",
                    &[
                        &(spirit_pid as i64),
                        &namespace_kind,
                        &namespace_detail,
                        &key,
                        &erased_at_source_ts,
                        &erased_at_source_region,
                    ],
                )
                .await?;
            transaction.commit().await?;
            return Ok(CollectiveEraseReceipt {
                deleted_rows: 0,
                tombstone_recorded: true,
            });
        }
        let deleted_rows = transaction
            .execute(
                "DELETE FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4
                   AND source_ts = $5 AND source_region = $6",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &namespace_detail,
                    &key,
                    &expected_source_ts,
                    &expected_source_region,
                ],
            )
            .await?;
        let now_ns = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        )
        .unwrap_or(i64::MAX);
        let (erased_at_source_ts, erased_at_source_region) = dominating_erasure_clock(
            Some((expected_source_ts, expected_source_region)),
            now_ns,
            &self.config.home_region,
        );
        transaction
            .execute(
                "INSERT INTO collective_erasure_tombstones
                    (spirit_pid, namespace_kind, namespace_detail, key,
                     erased_at_source_ts, erased_at_source_region)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                 DO UPDATE SET
                     erased_at_source_ts = EXCLUDED.erased_at_source_ts,
                     erased_at_source_region = EXCLUDED.erased_at_source_region,
                     created_at = NOW()
                 WHERE EXCLUDED.erased_at_source_ts >
                           collective_erasure_tombstones.erased_at_source_ts
                    OR (EXCLUDED.erased_at_source_ts =
                           collective_erasure_tombstones.erased_at_source_ts
                        AND EXCLUDED.erased_at_source_region >
                           collective_erasure_tombstones.erased_at_source_region)",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &namespace_detail,
                    &key,
                    &erased_at_source_ts,
                    &erased_at_source_region,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(CollectiveEraseReceipt {
            deleted_rows,
            tombstone_recorded: true,
        })
    }

    /// Erase exactly the crossed physical row received from `source_team`.
    ///
    /// The expected generation is checked while holding the writer's advisory
    /// lock and is repeated in the DELETE predicate, so a later generation
    /// cannot be removed by a reconciliation for an earlier share.
    pub async fn erase_crossed_row(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        key: &str,
        source_team: &TeamId,
        expected_source_ts: i64,
        expected_source_region: &str,
    ) -> Result<CollectiveEraseReceipt, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let (namespace_kind, logical_namespace_detail) = schema::namespace_to_parts(namespace);
        let physical_namespace_detail =
            cross_team_namespace_detail(source_team, &logical_namespace_detail);
        let lock_key = erasure_lock_key(spirit_pid, namespace_kind, &logical_namespace_detail, key);
        let mut client = self.get_client_with_timeout().await?;
        let transaction = client.transaction().await?;
        transaction
            .query_one(
                "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
                &[&lock_key],
            )
            .await?;
        let row = transaction
            .query_opt(
                "SELECT source_ts, source_region
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4 AND source_team = $5
                 FOR UPDATE",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &physical_namespace_detail,
                    &key,
                    &source_team.as_str(),
                ],
            )
            .await?;
        if let Some(row) = row.as_ref() {
            if row.get::<_, i64>(0) != expected_source_ts
                || row.get::<_, String>(1) != expected_source_region
            {
                return Err(StoreError::StaleGeneration);
            }
        }
        let row_erasure_clock = row
            .as_ref()
            .map(|row| (row.get::<_, i64>(0), row.get::<_, String>(1)));
        let deleted_rows = transaction
            .execute(
                "DELETE FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND namespace_detail = $3 AND key = $4 AND source_team = $5
                   AND source_ts = $6 AND source_region = $7",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &physical_namespace_detail,
                    &key,
                    &source_team.as_str(),
                    &expected_source_ts,
                    &expected_source_region,
                ],
            )
            .await?;
        let now_ns = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        )
        .unwrap_or(i64::MAX);
        let (erased_at_source_ts, erased_at_source_region) = dominating_erasure_clock(
            row_erasure_clock
                .as_ref()
                .map(|(source_ts, source_region)| (*source_ts, source_region.as_str())),
            now_ns,
            &self.config.home_region,
        );
        transaction
            .execute(
                "INSERT INTO collective_erasure_tombstones
                    (spirit_pid, namespace_kind, namespace_detail, key,
                     erased_at_source_ts, erased_at_source_region)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (spirit_pid, namespace_kind, namespace_detail, key)
                 DO UPDATE SET
                     erased_at_source_ts = EXCLUDED.erased_at_source_ts,
                     erased_at_source_region = EXCLUDED.erased_at_source_region,
                     created_at = NOW()
                 WHERE EXCLUDED.erased_at_source_ts >
                           collective_erasure_tombstones.erased_at_source_ts
                    OR (EXCLUDED.erased_at_source_ts =
                           collective_erasure_tombstones.erased_at_source_ts
                        AND EXCLUDED.erased_at_source_region >
                           collective_erasure_tombstones.erased_at_source_region)",
                &[
                    &(spirit_pid as i64),
                    &namespace_kind,
                    &physical_namespace_detail,
                    &key,
                    &erased_at_source_ts,
                    &erased_at_source_region,
                ],
            )
            .await?;
        transaction.commit().await?;
        Ok(CollectiveEraseReceipt {
            deleted_rows,
            tombstone_recorded: true,
        })
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
        reject_principal_namespace(namespace)?;
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);
        let cross_team_detail_pattern = format!("xteam:%:{}", hex::encode(ns_detail.as_bytes()));
        let rows = client
            .query(
                "SELECT value_kind, value_data, source_region, source_log_ref,
                        source_ts, source_team, distillation_depth, intent_lineage,
                        leaf_canonical_hash, merkle_root, region_sig,
                        bundle_schema_version, inclusion_path, namespace_kind,
                        namespace_detail
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND (namespace_detail = $3 OR namespace_detail LIKE $5)
                   AND key = $4
                 ORDER BY (namespace_detail = $3) DESC, source_ts DESC, source_region DESC
                 LIMIT 1",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &key,
                    &cross_team_detail_pattern,
                ],
            )
            .await?;

        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let val_kind: &str = row.get(0);
        let val_data: Vec<u8> = row.get(1);
        let src_region: String = row.get(2);
        let src_log_ref: String = row.get(3);
        let source_ts: i64 = row.get(4);
        let source_team_raw: Option<String> = row.get(5);
        let source_team = source_team_raw
            .as_deref()
            .map(TeamId::new)
            .transpose()
            .map_err(|error| StoreError::Query(error.to_string()))?;
        let distillation_depth_raw: i32 = row.get(6);
        let intent_lineage_bytes: Option<Vec<u8>> = row.get(7);
        let (distillation_depth, intent_lineage) = decode_row_provenance(
            distillation_depth_raw,
            intent_lineage_bytes.as_deref(),
            source_team.is_some(),
        )?;
        let persisted_leaf_hash: Option<Vec<u8>> = row.get(8);
        let persisted_root: Option<Vec<u8>> = row.get(9);
        let persisted_region_sig: Option<Vec<u8>> = row.get(10);
        let persisted_schema_version: Option<i16> = row.get(11);
        let persisted_proof: Option<Vec<u8>> = row.get(12);
        let row_ns_kind: String = row.get(13);
        let row_ns_detail: String = row.get(14);
        if !self.region_guard(&src_region, &src_log_ref) {
            return Ok(None);
        }
        // Marker ⟺ provenance consistency: a well-formed cross-team marker
        // with NULL source_team bypasses every attestation check and must be
        // refused, never served (13.3 review).
        if source_team.is_none() {
            if let Some((claimed_team, _)) = parse_cross_team_marker(&row_ns_detail) {
                return Err(StoreError::AttestationInvalid {
                    team_id: claimed_team,
                    reason: "cross-team namespace marker with NULL source_team".to_string(),
                });
            }
        }
        let logical_ns_detail = source_team
            .as_ref()
            .and_then(|team| original_cross_team_namespace_detail(team, &row_ns_detail))
            .unwrap_or(row_ns_detail);
        let leaf = CollectiveKvLeaf {
            source_region: src_region,
            source_ts,
            spirit_pid: spirit_pid as i64,
            namespace_kind: row_ns_kind,
            namespace_detail: logical_ns_detail,
            key: key.to_string(),
            value_kind: val_kind.to_string(),
            value_data: val_data.clone(),
            source_team,
            distillation_depth,
            intent_lineage,
        };
        self.attestation_guard(
            &leaf,
            persisted_leaf_hash.as_deref(),
            persisted_root.as_deref(),
            persisted_region_sig.as_deref(),
            persisted_schema_version,
            persisted_proof.as_deref(),
        )?;
        let value =
            schema::parts_to_value(val_kind, &val_data).map_err(StoreError::Serialization)?;
        Ok(Some(value))
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
    ///
    /// Story 13.3 (review): one physical winner per logical key —
    /// `DISTINCT ON (key)` with first-party-then-LWW ordering (the same
    /// winner rule `read` applies via `LIMIT 1`), so a first-party row plus
    /// cross-team copies of one key never surface as duplicate
    /// [`MemoryEntry`] identities and duplicates never consume LIMIT slots.
    pub async fn scan(
        &self,
        spirit_pid: u32,
        namespace: &MemoryNamespace,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        self.team_guard(spirit_pid)?;
        reject_principal_namespace(namespace)?;
        let client = self.get_client_with_timeout().await?;

        let (ns_kind, ns_detail) = schema::namespace_to_parts(namespace);
        let cross_team_detail_pattern = format!("xteam:%:{}", hex::encode(ns_detail.as_bytes()));
        let escaped_prefix = prefix
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let like_pattern = format!("{escaped_prefix}%");

        let rows = client
            .query(
                "SELECT DISTINCT ON (key) namespace_kind, namespace_detail, key, value_kind,
                        value_data, timestamp_ns, source_region, source_log_ref,
                        source_ts, source_team, distillation_depth, intent_lineage,
                        leaf_canonical_hash, merkle_root, region_sig,
                        bundle_schema_version, inclusion_path
                 FROM collective_memory
                 WHERE spirit_pid = $1 AND namespace_kind = $2
                   AND (namespace_detail = $3 OR namespace_detail LIKE $7)
                   AND key LIKE $4
                   AND (source_region = $6 OR source_log_ref <> '')
                 ORDER BY key ASC, (namespace_detail = $3) DESC,
                        source_ts DESC, source_region DESC
                 LIMIT $5",
                &[
                    &(spirit_pid as i64),
                    &ns_kind,
                    &ns_detail,
                    &like_pattern,
                    &(limit as i64),
                    &self.config.home_region.as_str(),
                    &cross_team_detail_pattern,
                ],
            )
            .await?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in &rows {
            let row_ns_kind: String = row.get(0);
            let row_ns_detail: String = row.get(1);
            let row_key: String = row.get(2);
            let row_val_kind: String = row.get(3);
            let row_val_data: Vec<u8> = row.get(4);
            let row_ts: i64 = row.get(5);
            let src_region: String = row.get(6);
            let src_log_ref: String = row.get(7);
            let source_ts: i64 = row.get(8);
            let source_team_raw: Option<String> = row.get(9);
            let source_team = source_team_raw
                .as_deref()
                .map(TeamId::new)
                .transpose()
                .map_err(|error| StoreError::Query(error.to_string()))?;
            let distillation_depth_raw: i32 = row.get(10);
            let intent_lineage_bytes: Option<Vec<u8>> = row.get(11);
            let (distillation_depth, intent_lineage) = decode_row_provenance(
                distillation_depth_raw,
                intent_lineage_bytes.as_deref(),
                source_team.is_some(),
            )?;
            let persisted_leaf_hash: Option<Vec<u8>> = row.get(12);
            let persisted_root: Option<Vec<u8>> = row.get(13);
            let persisted_region_sig: Option<Vec<u8>> = row.get(14);
            let persisted_schema_version: Option<i16> = row.get(15);
            let persisted_proof: Option<Vec<u8>> = row.get(16);

            if !self.region_guard(&src_region, &src_log_ref) {
                continue;
            }
            // Same marker ⟺ provenance consistency refusal as `read`.
            if source_team.is_none() {
                if let Some((claimed_team, _)) = parse_cross_team_marker(&row_ns_detail) {
                    return Err(StoreError::AttestationInvalid {
                        team_id: claimed_team,
                        reason: "cross-team namespace marker with NULL source_team".to_string(),
                    });
                }
            }
            let logical_ns_detail = source_team
                .as_ref()
                .and_then(|team| original_cross_team_namespace_detail(team, &row_ns_detail))
                .unwrap_or(row_ns_detail);
            let leaf = CollectiveKvLeaf {
                source_region: src_region,
                source_ts,
                spirit_pid: spirit_pid as i64,
                namespace_kind: row_ns_kind.clone(),
                namespace_detail: logical_ns_detail.clone(),
                key: row_key.clone(),
                value_kind: row_val_kind.clone(),
                value_data: row_val_data.clone(),
                source_team,
                distillation_depth,
                intent_lineage,
            };
            self.attestation_guard(
                &leaf,
                persisted_leaf_hash.as_deref(),
                persisted_root.as_deref(),
                persisted_region_sig.as_deref(),
                persisted_schema_version,
                persisted_proof.as_deref(),
            )?;

            let ns = schema::parts_to_namespace(&row_ns_kind, &logical_ns_detail)
                .map_err(StoreError::Serialization)?;
            let val = schema::parts_to_value(&row_val_kind, &row_val_data)
                .map_err(StoreError::Serialization)?;
            let entry = MemoryEntry::new(ns, row_key, val, row_ts as u64)
                .map_err(|error| StoreError::Serialization(format!("invalid entry: {error}")))?;
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
        let home_team =
            TeamId::new(&self.config.home_team).map_err(|error| StoreError::TenantMapStale {
                team_id: None,
                reason: error.to_string(),
            })?;
        let tenant_map = self
            .tenant_map
            .as_ref()
            .ok_or_else(|| StoreError::TenantMapStale {
                team_id: Some(home_team.clone()),
                reason: "tenant map is not configured".to_string(),
            })?;
        let expected_database =
            tenant_map
                .datname_for(&home_team)
                .map_err(|error| StoreError::TenantMapStale {
                    team_id: Some(home_team.clone()),
                    reason: error.to_string(),
                })?;
        if expected_database != current_database {
            return Err(StoreError::TenantConnectionMismatch {
                configured_team: home_team,
                caller_team: None,
                reason: format!(
                    "expected database {expected_database}, connected to {current_database}"
                ),
            });
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
        let configured_team =
            TeamId::new(&self.config.home_team).map_err(|error| StoreError::TenantMapStale {
                team_id: None,
                reason: error.to_string(),
            })?;
        let tenant_map = self
            .tenant_map
            .as_ref()
            .ok_or_else(|| StoreError::TenantMapStale {
                team_id: Some(configured_team.clone()),
                reason: "tenant map is not configured".to_string(),
            })?;
        let caller_team = tenant_map
            .team_of(spirit_pid)
            .map_err(|error| match error {
                TenantMapError::Stale { reason } | TenantMapError::StateUnavailable { reason } => {
                    StoreError::TenantMapStale {
                        team_id: Some(configured_team.clone()),
                        reason,
                    }
                }
                TenantMapError::SpiritUnmapped { spirit_pid } => {
                    StoreError::TenantSpiritUnmapped { spirit_pid }
                }
                TenantMapError::TeamUnknown { team_id } => StoreError::TenantMapStale {
                    reason: format!("team {team_id} is absent from the verified tenant map"),
                    team_id: Some(team_id),
                },
            })?;
        if caller_team != configured_team {
            return Err(StoreError::TenantConnectionMismatch {
                configured_team,
                caller_team: Some(caller_team),
                reason: "caller team differs from the store home team".to_string(),
            });
        }
        Ok(())
    }

    /// Verify the team-axis row attestation after query and before serving.
    ///
    /// The selected plaintext posture intentionally recomputes the canonical
    /// leaf hash from the exact persisted row. When an at-rest seal is
    /// configured, cross-team reads fail closed until Story 13.5a supplies the
    /// missing unseal path; first-party rows remain unaffected.
    #[allow(clippy::too_many_arguments)]
    fn attestation_guard(
        &self,
        leaf: &CollectiveKvLeaf,
        persisted_leaf_hash: Option<&[u8]>,
        persisted_root: Option<&[u8]>,
        persisted_region_sig: Option<&[u8]>,
        persisted_schema_version: Option<i16>,
        persisted_proof: Option<&[u8]>,
    ) -> Result<(), StoreError> {
        let Some(source_team) = leaf.source_team.as_ref() else {
            return Ok(());
        };
        let invalid = |reason: &str| StoreError::AttestationInvalid {
            team_id: source_team.clone(),
            reason: reason.to_string(),
        };
        if self.at_rest_sealer.is_configured() {
            return Err(invalid("configured seal has no paired unseal path"));
        }
        let (
            Some(persisted_leaf_hash),
            Some(persisted_root),
            Some(persisted_region_sig),
            Some(persisted_schema_version),
            Some(persisted_proof),
        ) = (
            persisted_leaf_hash,
            persisted_root,
            persisted_region_sig,
            persisted_schema_version,
            persisted_proof,
        )
        else {
            return Err(invalid("required attestation column is NULL"));
        };
        let leaf_hash = <[u8; 32]>::try_from(persisted_leaf_hash)
            .map_err(|_| invalid("leaf hash is not 32 bytes"))?;
        let root = <[u8; 32]>::try_from(persisted_root)
            .map_err(|_| invalid("Merkle root is not 32 bytes"))?;
        let schema_version = u16::try_from(persisted_schema_version)
            .map_err(|_| invalid("bundle schema version is negative"))?;
        let proof = serde_json::from_slice::<MerkleProof>(persisted_proof)
            .map_err(|_| invalid("inclusion proof is malformed"))?;
        if proof.adjacent_leaf.is_some()
            || proof.leaf != leaf_hash
            || leaf.canonical_hash() != leaf_hash
            || !verify_proof(root, leaf_hash, &proof)
        {
            return Err(invalid("row binding or Merkle inclusion failed"));
        }
        let source_region = Region::canonicalize(&leaf.source_region)
            .map_err(|_| invalid("source region is not canonical"))?;
        let public_key = self
            .team_verifying_keys
            .iter()
            .find_map(|((region, team), key)| {
                (region == &source_region && team == source_team).then_some(key)
            })
            .ok_or_else(|| invalid("no public key for the claimed region and team"))?;
        if !verify_team_root_signature(
            schema_version,
            &source_region,
            source_team,
            &root,
            persisted_region_sig,
            public_key,
        ) {
            return Err(invalid("root signature verification failed"));
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
                        source_team, distillation_depth, intent_lineage,
                        cross_emitter_host, cross_op_id, cross_source_ts, cross_source_region
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
            let distillation_depth_raw: i32 = row.get(10);
            let intent_lineage_bytes: Option<Vec<u8>> = row.get(11);
            let (distillation_depth, intent_lineage) = decode_row_provenance(
                distillation_depth_raw,
                intent_lineage_bytes.as_deref(),
                source_team.is_some(),
            )?;
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
                distillation_depth,
                intent_lineage,
                cross_emitter_host: row.get(12),
                cross_op_id: row.get(13),
                cross_source_ts: row.get(14),
                cross_source_region: row.get(15),
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
    /// Story 13.3b: `None` preserves v1/v2; v3 carries both fields together.
    pub distillation_depth: Option<u32>,
    pub intent_lineage: Option<IntentLineage>,
    /// Nullable share-operation binding copied only onto crossed rows.
    pub cross_emitter_host: Option<String>,
    pub cross_op_id: Option<String>,
    pub cross_source_ts: Option<i64>,
    pub cross_source_region: Option<String>,
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

    #[test]
    fn row_provenance_decoder_enforces_complete_v3_shape() {
        let encoded = br#"["schema.review","collective:share"]"#;
        let (depth, lineage) = decode_row_provenance(2, Some(encoded), true).unwrap();
        assert_eq!(depth, Some(2));
        assert_eq!(
            lineage.unwrap().as_slice(),
            [
                maos_domain::invariants::i8::A2AIntent::new("schema.review"),
                maos_domain::invariants::i8::A2AIntent::new("collective:share"),
            ]
        );
        assert!(decode_row_provenance(0, None, false).unwrap().0.is_none());
        assert!(decode_row_provenance(2, None, true).is_err());
        assert!(decode_row_provenance(0, Some(encoded), true).is_err());
        assert!(decode_row_provenance(-1, Some(encoded), true).is_err());
        // Provenance without a source team is a consistency violation.
        assert!(decode_row_provenance(2, Some(encoded), false).is_err());
    }

    #[test]
    fn erasure_tombstone_clock_blocks_stale_and_equal_replication() {
        assert!(erasure_clock_dominates(20, "region-a", 19, "region-z"));
        assert!(erasure_clock_dominates(20, "region-b", 20, "region-a"));
        assert!(erasure_clock_dominates(20, "region-a", 20, "region-a"));
        assert!(!erasure_clock_dominates(20, "region-a", 21, "region-a"));
        assert!(!erasure_clock_dominates(20, "region-a", 20, "region-b"));
    }

    #[test]
    fn erasure_clock_captures_future_dated_leaf() {
        let (erased_at_source_ts, erased_at_source_region) =
            dominating_erasure_clock(Some((21, "region-a")), 20, "home-region");

        assert_eq!(
            (erased_at_source_ts, erased_at_source_region.as_str()),
            (21, "region-a")
        );
        assert!(erasure_clock_dominates(
            erased_at_source_ts,
            &erased_at_source_region,
            21,
            "region-a"
        ));
    }

    #[tokio::test]
    async fn principal_namespace_is_refused_at_collective_store_entry() {
        let store = LoomLiteStore::new(StoreConfig {
            connection_string: "host=127.0.0.1 port=1 dbname=none connect_timeout=1".into(),
            timeout_ms: 25,
            ..Default::default()
        })
        .await
        .expect("construct lazy store");
        let namespace = MemoryNamespace::Principal {
            principal_id: "user-42".into(),
            schema: "profile.v1".into(),
        };

        let error = store
            .write(
                7,
                &namespace,
                "must-not-land",
                MemoryValue::Text("pii".into()),
            )
            .await
            .expect_err("collective store must refuse principal namespace");
        assert!(matches!(
            error,
            StoreError::PrincipalNamespaceForbidden { .. }
        ));
    }

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

    #[test]
    fn cross_team_namespace_details_are_distinct_and_stable() {
        let team_a = TeamId::new("team-a").unwrap();
        let team_b = TeamId::new("team-b").unwrap();
        let first_party = String::new();
        let a = cross_team_namespace_detail(&team_a, &first_party);
        let b = cross_team_namespace_detail(&team_b, &first_party);
        assert_ne!(a, first_party);
        assert_ne!(a, b);
        assert_eq!(a, cross_team_namespace_detail(&team_a, &first_party));
    }

    #[test]
    fn crossed_marker_preserves_logical_detail() {
        let team = TeamId::new("team-a").unwrap();
        let detail = cross_team_namespace_detail(&team, "detail");
        assert_eq!(
            original_cross_team_namespace_detail(&team, &detail),
            Some("detail".to_string())
        );
        assert_eq!(detail, "xteam:team-a:64657461696c");
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
            Err(StoreError::TenantMapStale { .. })
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
            Err(StoreError::TenantMapStale { .. })
        ));
        assert!(matches!(
            stale.scan(7, &MemoryNamespace::Default, "k", 8).await,
            Err(StoreError::TenantMapStale { .. })
        ));

        let unmapped = LoomLiteStore::new(dead())
            .await
            .unwrap()
            .with_tenant_map(Arc::new(UnmappedTenantMap));
        assert!(matches!(
            unmapped.read(9, &MemoryNamespace::Default, "k").await,
            Err(StoreError::TenantSpiritUnmapped { spirit_pid: 9 })
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
            Err(StoreError::TenantSpiritUnmapped { spirit_pid: 9 })
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
            Err(StoreError::TenantConnectionMismatch { .. })
        ));
    }
}
