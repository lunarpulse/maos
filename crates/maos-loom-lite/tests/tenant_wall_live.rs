#![forbid(unsafe_code)]

//! Story 13.1 live Postgres witnesses for the physical tenant wall.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use maos_domain::memory::{MemoryNamespace, MemoryValue};
use maos_domain::ports::registry::SpiritId;
use maos_domain::team::TeamId;
use maos_loom_lite::replication::leaf::{
    compute_kv_payload_oracle, kv_merkle_root, CollectiveKvLeaf,
};
use maos_loom_lite::store::{LoomLiteStore, StoreConfig, StoreError};
use maos_loom_lite::tenant::{TenantMapError, TenantMapPort};
use tokio_postgres::NoTls;

static PG_LOCK: Mutex<()> = Mutex::new(());

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

/// Minimal exact-I/O fixture schema. The Story 13.1 gate intentionally uses
/// the required `postgres:16` CI image (without pgvector); vector/HNSW schema
/// installation is covered by the existing Loom live suite and is unrelated
/// to the tenant-wall witness.
async fn init_fixture_schema(client: &tokio_postgres::Client) {
    client
        .batch_execute(
            r#"
            CREATE TABLE IF NOT EXISTS collective_memory (
                id BIGSERIAL PRIMARY KEY,
                spirit_pid BIGINT NOT NULL,
                namespace_kind TEXT NOT NULL,
                namespace_detail TEXT NOT NULL DEFAULT '',
                key TEXT NOT NULL,
                value_kind TEXT NOT NULL,
                value_data BYTEA NOT NULL,
                kind TEXT NOT NULL DEFAULT 'entry',
                source_log_ref TEXT NOT NULL DEFAULT '',
                distillation_depth INTEGER NOT NULL DEFAULT 0,
                timestamp_ns BIGINT NOT NULL,
                source_region TEXT NOT NULL DEFAULT '',
                source_ts BIGINT NOT NULL DEFAULT 0,
                source_team TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                UNIQUE (spirit_pid, namespace_kind, namespace_detail, key),
                CONSTRAINT collective_memory_i11_provenance CHECK (
                    kind <> 'pattern' OR (source_log_ref <> '' AND distillation_depth > 0)
                )
            );
            "#,
        )
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
        Err(StoreError::TenantConnectionMismatch(_))
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
        Err(StoreError::TenantConnectionMismatch(_))
    ));

    // Patch 5: positive connection-assignment path against real Postgres.
    // store_a's home team maps to datname_a (== current_database), so
    // init_schema's self-check (store.rs:213-220) MUST pass. The subsequent
    // CREATE EXTENSION vector step may fail on the pgvector-less CI image, but
    // the assignment guard runs first — it must never be the refusal.
    if let Err(StoreError::TenantConnectionMismatch(reason)) = store_a.init_schema().await {
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
        Err(StoreError::TenantConnectionMismatch(_))
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
        Err(StoreError::TenantConnectionMismatch(_))
    ));
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
            1_700_000_000_000_000_003,
            "region-b",
            "forged-non-empty-log-ref",
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
    let store_a = make_store(team_a_conn(), "team-a", "region-a", map.clone()).await;
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
    // team-A: a re-attested v2 cross-team COPY (source_team = 'team-c'), inserted
    // via raw SQL — 13.2 has no production path that writes source_team (that is
    // 13.3); this synthesizes the mixed store a read would see.
    raw_a
        .execute(
            "INSERT INTO collective_memory
                (spirit_pid, namespace_kind, namespace_detail, key, value_kind, value_data,
                 timestamp_ns, source_ts, source_region, source_log_ref, source_team)
             VALUES (9, 'default', '', 'c-copy', 'text', $1, 0, 5, 'region-c', 'stamp', 'team-c')",
            &[&b"cross-team-copy".to_vec()],
        )
        .await
        .expect("v2 cross-team copy insert");

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
