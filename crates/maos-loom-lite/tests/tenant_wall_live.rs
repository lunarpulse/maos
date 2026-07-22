#![forbid(unsafe_code)]

//! Story 13.1 live Postgres witnesses for the physical tenant wall.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use maos_audit::sealed_export::derive_team_pubkey;
use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::ports::registry::SpiritId;
use maos_domain::region::Region;
use maos_domain::team::TeamId;
use maos_loom_lite::cross_team_consent::{CrossTeamConsentError, CrossTeamConsentPort};
use maos_loom_lite::replication::bundle::{
    apply_replication_bundle, build_replication_bundle_v2, CrossTeamApplyContext,
};
use maos_loom_lite::replication::leaf::{
    compute_kv_payload_oracle, kv_merkle_root, CollectiveKvLeaf,
};
use maos_loom_lite::store::{LoomLiteStore, StoreConfig, StoreError};
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};
use tokio_postgres::NoTls;

static PG_LOCK: Mutex<()> = Mutex::new(());

const TEAM_BASE_SEED: [u8; 32] = [0x42; 32];

struct TeamCToAConsent;

impl CrossTeamConsentPort for TeamCToAConsent {
    fn is_granted(
        &self,
        from_team: &TeamId,
        to_team: &TeamId,
        intent: &str,
    ) -> Result<bool, CrossTeamConsentError> {
        Ok(from_team.as_str() == "team-c"
            && to_team.as_str() == "team-a"
            && intent == "collective:share")
    }
}

fn guard() -> std::sync::MutexGuard<'static, ()> {
    PG_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

fn team_a_conn() -> String {
    std::env::var("MAOS_TEST_POSTGRES_TEAM_A")
        .expect("MAOS_TEST_POSTGRES_TEAM_A must be set for tenant_wall_live")
}

fn team_b_conn() -> String {
    std::env::var("MAOS_TEST_POSTGRES_TEAM_B")
        .expect("MAOS_TEST_POSTGRES_TEAM_B must be set for tenant_wall_live")
}

async fn raw_connect(connection_string: &str) -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
        .await
        .expect("raw Postgres connection");
    tokio::spawn(async move {
        let _ = connection.await;
    });
    client
}

async fn current_database(client: &tokio_postgres::Client) -> String {
    client
        .query_one("SELECT current_database()", &[])
        .await
        .expect("current_database query")
        .get(0)
}

/// Install the PRODUCTION schema, not a hand-rolled copy of it.
///
/// This used to inline a reduced `CREATE TABLE collective_memory`, because the
/// Story 13.1 gate ran on the plain `postgres:16` CI image which has no
/// pgvector and so could not execute `CREATE EXTENSION vector`. That copy then
/// drifted: Story 13.3b added the `intent_lineage` column to the real schema
/// and to the store's INSERT, but not here, so every write in this suite died
/// with `Query("db error")` the moment the gate first ran in CI.
///
/// The CI image is now `pgvector/pgvector:pg16`, which removes the reason the
/// copy existed. Calling `create_schema_sql` means the fixture cannot drift
/// from production again — there is only one schema.
async fn init_fixture_schema(client: &tokio_postgres::Client) {
    client
        .batch_execute(&maos_loom_lite::schema::create_schema_sql(
            maos_loom_lite::schema::DEFAULT_VECTOR_DIM,
        ))
        .await
        .expect("tenant-wall fixture schema");
}

struct LiveTenantMap {
    bindings: Mutex<HashMap<u32, SpiritId>>,
    datnames: HashMap<TeamId, String>,
}

impl LiveTenantMap {
    fn new(team_a_datname: String, team_b_datname: String) -> Self {
        Self {
            bindings: Mutex::new(HashMap::new()),
            datnames: HashMap::from([
                (TeamId::new("team-a").unwrap(), team_a_datname),
                (TeamId::new("team-b").unwrap(), team_b_datname),
            ]),
        }
    }
}

impl TenantMapPort for LiveTenantMap {
    fn team_of(&self, spirit_pid: u32) -> Result<TeamId, TenantMapError> {
        let spirit_id = self
            .bindings
            .lock()
            .map_err(|_| TenantMapError::StateUnavailable {
                reason: "test binding lock poisoned".to_string(),
            })?
            .get(&spirit_pid)
            .cloned()
            .ok_or(TenantMapError::SpiritUnmapped { spirit_pid })?;
        match spirit_id.as_str() {
            "spirit-a" => Ok(TeamId::new("team-a").unwrap()),
            "spirit-b" => Ok(TeamId::new("team-b").unwrap()),
            _ => Err(TenantMapError::SpiritUnmapped { spirit_pid }),
        }
    }

    fn datname_for(&self, team: &TeamId) -> Result<String, TenantMapError> {
        self.datnames
            .get(team)
            .cloned()
            .ok_or_else(|| TenantMapError::TeamUnknown {
                team_id: team.clone(),
            })
    }

    fn register_spirit(&self, spirit_pid: u32, spirit_id: SpiritId) {
        if let Ok(mut bindings) = self.bindings.lock() {
            bindings.insert(spirit_pid, spirit_id);
        }
    }
}

async fn make_store(
    connection_string: String,
    home_team: &str,
    home_region: &str,
    map: Arc<LiveTenantMap>,
) -> LoomLiteStore {
    LoomLiteStore::new(StoreConfig {
        connection_string,
        home_region: home_region.to_string(),
        home_team: home_team.to_string(),
        ..StoreConfig::default()
    })
    .await
    .expect("lazy store construction")
    .with_tenant_map(map)
}
#[test]
fn schema_initialization_wraps_ddl_in_advisory_lock() {
    let source = include_str!("../src/store.rs");
    let init_schema = source
        .split_once("pub async fn init_schema(&self) -> Result<(), StoreError> {")
        .expect("init_schema exists")
        .1
        .split_once("\n    /// Write a value")
        .expect("init_schema body ends before write")
        .0;
    let lock = init_schema
        .find("SELECT pg_advisory_lock($1)")
        .expect("schema initialization takes an advisory lock");
    let ddl = init_schema
        .find("let schema_result = client.batch_execute(&sql).await;")
        .expect("schema initialization executes the DDL batch");
    let unlock = init_schema
        .find("SELECT pg_advisory_unlock($1)")
        .expect("schema initialization releases the advisory lock");

    assert!(
        lock < ddl && ddl < unlock,
        "schema DDL must execute while its advisory lock is held"
    );
}

/// P8: independently pooled callers must serialize the additive DDL batch.
#[tokio::test]
#[ignore = "requires MAOS_TEST_POSTGRES_TEAM_A with pgvector"]
async fn schema_initialization_is_serialized_across_pool_connections() {
    let _guard = guard();
    let raw = raw_connect(&team_a_conn()).await;
    init_fixture_schema(&raw).await;
    raw.batch_execute(
        r#"
        CREATE EXTENSION IF NOT EXISTS vector;
        DROP INDEX IF EXISTS idx_collective_memory_embedding;
        DROP INDEX IF EXISTS idx_collective_memory_spirit_key;
        ALTER TABLE collective_memory DROP COLUMN IF EXISTS embedding;
        "#,
    )
    .await
    .expect("reset schema-init migration surface");

    let store = Arc::new(
        LoomLiteStore::new(StoreConfig {
            connection_string: team_a_conn(),
            pool_size: 4,
            ..StoreConfig::default()
        })
        .await
        .expect("lazy store construction"),
    );
    let mut calls = Vec::new();
    for _ in 0..4 {
        let store = Arc::clone(&store);
        calls.push(tokio::spawn(async move { store.init_schema().await }));
    }
    for call in calls {
        call.await
            .expect("schema initialization task did not panic")
            .expect("concurrent schema initialization succeeds");
    }

    let embedding_exists: bool = raw
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = current_schema()
                    AND table_name = 'collective_memory'
                    AND column_name = 'embedding'
            )",
            &[],
        )
        .await
        .expect("embedding-column query")
        .get(0);
    let index_exists: bool = raw
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = current_schema()
                    AND tablename = 'collective_memory'
                    AND indexname = 'idx_collective_memory_embedding'
            )",
            &[],
        )
        .await
        .expect("embedding-index query")
        .get(0);

    assert!(embedding_exists, "concurrent initialization adds embedding");
    assert!(index_exists, "concurrent initialization creates HNSW index");
}

#[tokio::test]
#[ignore = "requires two live Postgres databases"]
async fn tenant_wall_two_datname_physical_absence_and_assignment_matrix() {
    let _guard = guard();
    let raw_a = raw_connect(&team_a_conn()).await;
    let raw_b = raw_connect(&team_b_conn()).await;
    init_fixture_schema(&raw_a).await;
    init_fixture_schema(&raw_b).await;
    let datname_a = current_database(&raw_a).await;
    let datname_b = current_database(&raw_b).await;
    assert_ne!(
        datname_a, datname_b,
        "physical witness requires two databases"
    );

    let map = Arc::new(LiveTenantMap::new(datname_a.clone(), datname_b.clone()));
    map.register_spirit(7, SpiritId::from("spirit-a"));
    map.register_spirit(8, SpiritId::from("spirit-b"));

    let store_a = make_store(team_a_conn(), "team-a", "region-a", map.clone()).await;
    let store_b = make_store(team_b_conn(), "team-b", "region-b", map.clone()).await;
    raw_a
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();
    raw_b
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();

    store_a
        .write(
            7,
            &MemoryNamespace::Default,
            "team-a-only",
            MemoryValue::Text("physically isolated".to_string()),
        )
        .await
        .expect("same-team write admitted");
    assert!(matches!(
        store_a
            .write(
                8,
                &MemoryNamespace::Default,
                "wrong-team",
                MemoryValue::Text("must refuse".to_string()),
            )
            .await,
        Err(StoreError::TenantConnectionMismatch { .. })
    ));

    // Patch 4: reverse direction — a team-a spirit (7) writing through team-b's
    // store must ALSO refuse. Proves the operand resolution is not one-directional
    // (a home_team/caller_team swap would pass the forward case above but red here).
    assert!(matches!(
        store_b
            .write(
                7,
                &MemoryNamespace::Default,
                "wrong-team-reverse",
                MemoryValue::Text("must refuse".to_string()),
            )
            .await,
        Err(StoreError::TenantConnectionMismatch { .. })
    ));

    // Patch 5: positive connection-assignment path against real Postgres.
    // store_a's home team maps to datname_a (== current_database), so
    // init_schema's self-check (store.rs:213-220) MUST pass. The subsequent
    // CREATE EXTENSION vector step may fail on the pgvector-less CI image, but
    // the assignment guard runs first — it must never be the refusal.
    if let Err(StoreError::TenantConnectionMismatch { reason, .. }) = store_a.init_schema().await {
        panic!("positive connection-assignment must not refuse: {reason}");
    }

    let rows_a = LoomLiteStore::read_all_rows_from(&raw_a).await.unwrap();
    let rows_b = LoomLiteStore::read_all_rows_from(&raw_b).await.unwrap();
    assert!(rows_a.iter().any(|row| row.key == "team-a-only"));
    assert!(
        rows_b.iter().all(|row| row.key != "team-a-only"),
        "team A row must be physically absent from team B database"
    );

    let mismatch_a = LoomLiteStore::new(StoreConfig {
        connection_string: team_a_conn(),
        home_team: "team-b".to_string(),
        ..StoreConfig::default()
    })
    .await
    .unwrap()
    .with_tenant_map(map.clone());
    assert!(matches!(
        mismatch_a.init_schema().await,
        Err(StoreError::TenantConnectionMismatch { .. })
    ));
    let mismatch_b = LoomLiteStore::new(StoreConfig {
        connection_string: team_b_conn(),
        home_team: "team-a".to_string(),
        ..StoreConfig::default()
    })
    .await
    .unwrap()
    .with_tenant_map(map);
    assert!(matches!(
        mismatch_b.init_schema().await,
        Err(StoreError::TenantConnectionMismatch { .. })
    ));
}

/// Story 13.5d AC7 — registration makes the route serve exactly one team.
///
/// The positive row must exist in team A and be physically absent from team B.
/// A second, unregistered pid remains fail-closed.
#[tokio::test]
#[ignore = "requires two live Postgres databases"]
async fn spirit_collective_route_registered_pid_serves_only_own_team() {
    let _guard = guard();
    let raw_a = raw_connect(&team_a_conn()).await;
    let raw_b = raw_connect(&team_b_conn()).await;
    init_fixture_schema(&raw_a).await;
    init_fixture_schema(&raw_b).await;
    let map = Arc::new(LiveTenantMap::new(
        current_database(&raw_a).await,
        current_database(&raw_b).await,
    ));
    map.register_spirit(7, SpiritId::from("spirit-a"));
    map.register_spirit(8, SpiritId::from("spirit-b"));
    let store_a = make_store(team_a_conn(), "team-a", "region-a", map.clone()).await;

    raw_a
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();
    raw_b
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();

    store_a
        .write(
            7,
            &MemoryNamespace::Default,
            "story-13-5d-route",
            MemoryValue::Text("registered route served".to_string()),
        )
        .await
        .expect("registered pid writes to its mapped team");
    assert!(matches!(
        store_a
            .write(
                9,
                &MemoryNamespace::Default,
                "story-13-5d-unregistered",
                MemoryValue::Text("must not land".to_string()),
            )
            .await,
        Err(StoreError::TenantSpiritUnmapped { spirit_pid: 9 })
    ));

    let rows_a = LoomLiteStore::read_all_rows_from(&raw_a).await.unwrap();
    let rows_b = LoomLiteStore::read_all_rows_from(&raw_b).await.unwrap();
    assert!(
        rows_a.iter().any(|row| row.key == "story-13-5d-route"),
        "registered route must persist one row in team A"
    );
    assert!(
        rows_b.iter().all(|row| row.key != "story-13-5d-route"),
        "team A route row must be physically absent from team B"
    );
    assert!(
        rows_a
            .iter()
            .all(|row| row.key != "story-13-5d-unregistered")
            && rows_b
                .iter()
                .all(|row| row.key != "story-13-5d-unregistered"),
        "unregistered pid must write zero rows in either tenant"
    );
}

#[tokio::test]
#[ignore = "requires two live Postgres databases"]
async fn tenant_wall_d1_forged_stamp_is_still_served_boundary() {
    let _guard = guard();
    let raw_a = raw_connect(&team_a_conn()).await;
    let raw_b = raw_connect(&team_b_conn()).await;
    init_fixture_schema(&raw_a).await;
    init_fixture_schema(&raw_b).await;
    let map = Arc::new(LiveTenantMap::new(
        current_database(&raw_a).await,
        current_database(&raw_b).await,
    ));
    map.register_spirit(7, SpiritId::from("spirit-a"));
    let store_a = make_store(team_a_conn(), "team-a", "region-a", map).await;
    raw_a
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();

    store_a
        .write_with_source(
            7,
            &MemoryNamespace::Default,
            "forged-stamp",
            MemoryValue::Text("presence-only boundary".to_string()),
            maos_loom_lite::store::WriteSource {
                ts: 1_700_000_000_000_000_003,
                region: "region-b",
                log_ref: "forged-non-empty-log-ref",
                team: None,
                distillation_depth: None,
                intent_lineage: None,
            },
        )
        .await
        .expect("infrastructure injection demonstrates staged boundary");

    let served = store_a
        .read(7, &MemoryNamespace::Default, "forged-stamp")
        .await
        .expect("same-team read executes")
        .expect("Story 13.1 must still serve the forged presence stamp");
    assert_eq!(
        served,
        MemoryValue::Text("presence-only boundary".to_string())
    );
}

/// Story 13.2 (AC4 / AC5c): per-team Merkle independence over two physical
/// datnames, including the MIXED v1/v2 case. Team-A's store holds its own v1
/// rows (source_team NULL) plus a re-attested v2 copy (source_team = 'team-c');
/// team-B's store is disjoint. The `{root, payload-oracle, row-count}` triple of
/// team-A must be UNCHANGED by any mutation of team-B (physical database-per-team
/// makes this true by construction — AC4 proves it with the discriminating
/// triple, not the dedup-blind SET-root alone).
#[tokio::test]
#[ignore = "requires two live Postgres databases"]
async fn tenant_wall_per_team_merkle_independence_mixed_v1_v2() {
    let _guard = guard();
    let raw_a = raw_connect(&team_a_conn()).await;
    let raw_b = raw_connect(&team_b_conn()).await;
    init_fixture_schema(&raw_a).await;
    init_fixture_schema(&raw_b).await;
    let map = Arc::new(LiveTenantMap::new(
        current_database(&raw_a).await,
        current_database(&raw_b).await,
    ));
    map.register_spirit(7, SpiritId::from("spirit-a"));
    map.register_spirit(8, SpiritId::from("spirit-b"));
    let team_a = TeamId::new("team-a").unwrap();
    let team_c = TeamId::new("team-c").unwrap();
    let region_c = Region::canonicalize("region-c").unwrap();
    let store_a = make_store(team_a_conn(), "team-a", "region-a", map.clone())
        .await
        .with_cross_team_consent(Arc::new(TeamCToAConsent))
        .with_team_verifying_keys(HashMap::from([(
            (region_c.clone(), team_c.clone()),
            derive_team_pubkey(&TEAM_BASE_SEED, &region_c, &team_c),
        )]));
    let store_b = make_store(team_b_conn(), "team-b", "region-b", map).await;
    raw_a
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();
    raw_b
        .execute("DELETE FROM collective_memory", &[])
        .await
        .unwrap();

    // team-A: two first-party v1 rows (source_team stays NULL).
    for key in ["a-own-1", "a-own-2"] {
        store_a
            .write(
                7,
                &MemoryNamespace::Default,
                key,
                MemoryValue::Text(format!("v-{key}")),
            )
            .await
            .expect("team-a v1 write");
    }
    // team-A: a verified v2 cross-team COPY through the production apply seam.
    let cross_team_leaf = CollectiveKvLeaf {
        source_region: "region-c".to_string(),
        source_ts: 5,
        spirit_pid: 9,
        namespace_kind: "default".to_string(),
        namespace_detail: String::new(),
        key: "c-copy".to_string(),
        value_kind: "text".to_string(),
        value_data: b"cross-team-copy".to_vec(),
        source_team: Some(team_c.clone()),
        distillation_depth: None,
        intent_lineage: None,
    };
    let cross_team_bundle =
        build_replication_bundle_v2(vec![cross_team_leaf], &region_c, &team_c, &TEAM_BASE_SEED)
            .expect("leaf origin matches the envelope");
    apply_replication_bundle(
        &cross_team_bundle,
        &store_a,
        "region-a",
        Some(CrossTeamApplyContext::new(&team_a, "collective:share")),
        &TEAM_BASE_SEED,
    )
    .await
    .expect("v2 cross-team copy apply");

    // team-B: a disjoint first-party row.
    store_b
        .write(
            8,
            &MemoryNamespace::Default,
            "b-own-1",
            MemoryValue::Text("v-b".to_string()),
        )
        .await
        .expect("team-b v1 write");

    let triple = |leaves: &[CollectiveKvLeaf]| {
        (
            kv_merkle_root(leaves),
            compute_kv_payload_oracle(leaves),
            leaves.len(),
        )
    };
    let leaves_a = |rows: &[maos_loom_lite::store::CollectiveRow]| {
        rows.iter()
            .map(CollectiveKvLeaf::from_row)
            .collect::<Vec<_>>()
    };

    let rows_a_before = LoomLiteStore::read_all_rows_from(&raw_a).await.unwrap();
    let leaves_before = leaves_a(&rows_a_before);
    assert_eq!(leaves_before.len(), 3, "team-A holds 2 v1 + 1 v2 row");
    assert!(
        leaves_before
            .iter()
            .any(|l| l.source_team.as_ref().map(|t| t.as_str()) == Some("team-c")),
        "the mixed store must include the v2 cross-team copy"
    );
    assert_eq!(
        leaves_before
            .iter()
            .filter(|l| l.source_team.is_none())
            .count(),
        2,
        "the two first-party rows stay v1 (source_team NULL)"
    );
    let triple_a_before = triple(&leaves_before);

    // Mutate team-B: overwrite its row + add another. Its own triple must move.
    let rows_b_before = LoomLiteStore::read_all_rows_from(&raw_b).await.unwrap();
    let triple_b_before = triple(&leaves_a(&rows_b_before));
    store_b
        .write(
            8,
            &MemoryNamespace::Default,
            "b-own-1",
            MemoryValue::Text("v-b-CHANGED".to_string()),
        )
        .await
        .expect("team-b mutation");
    store_b
        .write(
            8,
            &MemoryNamespace::Default,
            "b-own-2",
            MemoryValue::Text("v-b-2".to_string()),
        )
        .await
        .expect("team-b growth");
    let rows_b_after = LoomLiteStore::read_all_rows_from(&raw_b).await.unwrap();
    assert_ne!(
        triple_b_before,
        triple(&leaves_a(&rows_b_after)),
        "team-B mutation must move team-B's own triple"
    );

    // team-A's triple is UNCHANGED by the team-B mutation (physical isolation).
    let rows_a_after = LoomLiteStore::read_all_rows_from(&raw_a).await.unwrap();
    assert_eq!(
        triple_a_before,
        triple(&leaves_a(&rows_a_after)),
        "team-A's {{root, payload-oracle, row-count}} triple must be independent of team-B"
    );
}
