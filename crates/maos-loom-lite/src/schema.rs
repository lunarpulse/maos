#![forbid(unsafe_code)]

//! Postgres+pgvector schema management for the Loom-lite collective tier.
//!
//! Namespaced key-value store with a `vector(N)` column and HNSW index.
//! Records carry `kind: pattern` + `source_log_ref` / `distillation_depth`
//! per I11 for Loom-persisted patterns.

/// Default vector dimension for the pgvector column.
pub const DEFAULT_VECTOR_DIM: usize = 1536;

/// SQL DDL to create the collective tier schema.
///
/// Schema design:
/// - `id`: auto-generated primary key.
/// - `spirit_pid`: the writing Spirit's kernel-assigned pid.
/// - `namespace_kind`: discriminant of `MemoryNamespace` (e.g. "default", "coordination").
/// - `namespace_detail`: serialized namespace detail (principal_id, schema, etc.).
/// - `key`: the memory key (UTF-8, max 4096 bytes).
/// - `value_kind`: discriminant of `MemoryValue` ("json", "markdown", "blob", "text").
/// - `value_data`: the serialized value payload (BYTEA for Blob, TEXT for others).
/// - `embedding`: optional pgvector embedding for similarity search.
/// - `kind`: record kind — "pattern" for Loom-persisted patterns.
/// - `source_log_ref`: I11 provenance — TL frame_id hex of the source log entry.
/// - `distillation_depth`: I11 — number of distillation steps from raw observation.
/// - `timestamp_ns`: nanosecond timestamp (monotonic within Spirit).
/// - `created_at`: Postgres wall-clock timestamp.
pub fn create_schema_sql(vector_dim: usize) -> String {
    format!(
        r#"
CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS collective_memory (
    id              BIGSERIAL PRIMARY KEY,
    -- spirit_pid stored as BIGINT to avoid the u32->i32 narrowing that turns
    -- high pids into negative INTEGERs (AC1 review).
    spirit_pid      BIGINT NOT NULL,
    namespace_kind  TEXT NOT NULL,
    namespace_detail TEXT NOT NULL DEFAULT '',
    key             TEXT NOT NULL,
    value_kind      TEXT NOT NULL,
    -- value_data holds the serialized payload for ALL value kinds (BYTEA),
    -- not just Blob — the value_kind discriminant selects the deserializer.
    value_data      BYTEA NOT NULL,
    embedding       vector({vector_dim}),
    kind            TEXT NOT NULL DEFAULT 'entry',
    source_log_ref  TEXT NOT NULL DEFAULT '',
    distillation_depth INTEGER NOT NULL DEFAULT 0,
    timestamp_ns    BIGINT NOT NULL,
    -- Story 11.2a (AC1, F3): CRDT LWW-register total-order tiebreak columns.
    -- source_region: canonical ascii-v1 region tag of the originating write.
    -- source_ts: the source write's nanosecond timestamp, preserved across
    -- re-attestation apply (NOT re-minted on apply — re-minting destroys
    -- convergence).  Backfill: home region + 0 sentinel for pre-11.2a rows.
    -- These columns are NOT in the UNIQUE key (region-free convergence).
    source_region   TEXT NOT NULL DEFAULT '',
    source_ts       BIGINT NOT NULL DEFAULT 0,
    -- Story 13.2 (AC2): nullable source-team provenance. NULL = v1 first-party
    -- local row (byte-identical leaf); a value = re-attested cross-team copy.
    -- NOT in the UNIQUE key. Additive/nullable = the 9.2b idiom.
    source_team     TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (spirit_pid, namespace_kind, namespace_detail, key),
    -- I11 (enforced-by-construction at the store layer): a pattern record
    -- MUST carry non-empty source_log_ref + distillation_depth > 0.  Vacuous
    -- until patterns land (v1.5 ships KV-only; pattern-distillation is a
    -- named follow-up).  The CHECK makes a pattern-without-provenance insert
    -- a hard Postgres error.
    CONSTRAINT collective_memory_i11_provenance CHECK (
        kind <> 'pattern' OR (source_log_ref <> '' AND distillation_depth > 0)
    )
);

-- Story 11.2a: additive migration — add source_region and source_ts if absent.
-- Idempotent (IF NOT EXISTS / DO NOTHING on re-run).
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'collective_memory' AND column_name = 'source_region'
    ) THEN
        ALTER TABLE collective_memory ADD COLUMN source_region TEXT NOT NULL DEFAULT '';
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'collective_memory' AND column_name = 'source_ts'
    ) THEN
        ALTER TABLE collective_memory ADD COLUMN source_ts BIGINT NOT NULL DEFAULT 0;
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'collective_memory' AND column_name = 'source_team'
    ) THEN
        ALTER TABLE collective_memory ADD COLUMN source_team TEXT;
    END IF;
END
$$;

-- HNSW index for filtered similarity queries (pgvector 0.8+).  Staged schema:
-- v1.5 ships KV-only; the vector(N)/HNSW/pgvector surface is inert until a
-- pattern-retrieval/distillation story populates embeddings.
CREATE INDEX IF NOT EXISTS idx_collective_memory_embedding
    ON collective_memory USING hnsw (embedding vector_cosine_ops);

-- B-tree indexes for key-prefix scan and spirit lookup
CREATE INDEX IF NOT EXISTS idx_collective_memory_spirit_key
    ON collective_memory (spirit_pid, namespace_kind, key);
"#
    )
}

/// SQL to set pgvector HNSW session parameters on each connection.
pub const HNSW_SESSION_INIT: &str = r#"
SET hnsw.iterative_scan = 'relaxed_order';
SET hnsw.max_scan_tuples = 10000;
"#;

/// Serialize a `MemoryNamespace` into (kind, detail) for storage.
pub fn namespace_to_parts(ns: &maos_domain::memory::MemoryNamespace) -> (&'static str, String) {
    use maos_domain::memory::MemoryNamespace;
    match ns {
        MemoryNamespace::Default => ("default", String::new()),
        MemoryNamespace::Coordination => ("coordination", String::new()),
        MemoryNamespace::Forgotten => ("forgotten", String::new()),
        MemoryNamespace::Principal {
            principal_id,
            schema,
        } => ("principal", format!("{principal_id}:{schema}")),
    }
}

/// Deserialize (kind, detail) back into a `MemoryNamespace`.
///
/// The `principal` detail uses `principal_id:schema`.  The `:` delimiter is
/// GUARANTEED absent from validated keys because `PrincipalKey::new` rejects
/// `:` (and control chars), so `split_once(':')` round-trips losslessly for
/// any namespace constructed through the validated path.  (The collective tier
/// rejects `Principal` by construction per Decision D regardless.)
pub fn parts_to_namespace(
    kind: &str,
    detail: &str,
) -> Result<maos_domain::memory::MemoryNamespace, String> {
    use maos_domain::memory::MemoryNamespace;
    match kind {
        "default" => Ok(MemoryNamespace::Default),
        "coordination" => Ok(MemoryNamespace::Coordination),
        "forgotten" => Ok(MemoryNamespace::Forgotten),
        "principal" => {
            let (pid, schema) = detail
                .split_once(':')
                .ok_or_else(|| format!("invalid principal detail: {detail}"))?;
            Ok(MemoryNamespace::Principal {
                principal_id: pid.to_string(),
                schema: schema.to_string(),
            })
        }
        _ => Err(format!("unknown namespace kind: {kind}")),
    }
}

/// Serialize a `MemoryValue` into (kind, data) for storage.
///
/// Returns an error on JSON serialization failure rather than silently storing
/// empty bytes (AC1 review — no silent empty-byte corruption).
pub fn value_to_parts(
    val: &maos_domain::memory::MemoryValue,
) -> Result<(&'static str, Vec<u8>), String> {
    use maos_domain::memory::MemoryValue;
    match val {
        MemoryValue::Json(v) => serde_json::to_vec(v)
            .map(|b| ("json", b))
            .map_err(|e| format!("json encode: {e}")),
        MemoryValue::Markdown(s) => Ok(("markdown", s.as_bytes().to_vec())),
        MemoryValue::Blob(b) => Ok(("blob", b.clone())),
        MemoryValue::Text(s) => Ok(("text", s.as_bytes().to_vec())),
    }
}

/// Deserialize (kind, data) back into a `MemoryValue`.
pub fn parts_to_value(kind: &str, data: &[u8]) -> Result<maos_domain::memory::MemoryValue, String> {
    use maos_domain::memory::MemoryValue;
    match kind {
        "json" => {
            let v: serde_json::Value =
                serde_json::from_slice(data).map_err(|e| format!("json decode: {e}"))?;
            Ok(MemoryValue::Json(v))
        }
        "markdown" => Ok(MemoryValue::Markdown(
            String::from_utf8(data.to_vec()).map_err(|e| format!("utf8: {e}"))?,
        )),
        "blob" => Ok(MemoryValue::Blob(data.to_vec())),
        "text" => Ok(MemoryValue::Text(
            String::from_utf8(data.to_vec()).map_err(|e| format!("utf8: {e}"))?,
        )),
        _ => Err(format!("unknown value kind: {kind}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maos_domain::memory::{MemoryNamespace, MemoryValue};

    #[test]
    fn namespace_round_trip_default() {
        let ns = MemoryNamespace::Default;
        let (kind, detail) = namespace_to_parts(&ns);
        let recovered = parts_to_namespace(kind, &detail).unwrap();
        assert_eq!(ns, recovered);
    }

    #[test]
    fn namespace_round_trip_principal() {
        let ns = MemoryNamespace::Principal {
            principal_id: "user-42".into(),
            schema: "profile.v1".into(),
        };
        let (kind, detail) = namespace_to_parts(&ns);
        let recovered = parts_to_namespace(kind, &detail).unwrap();
        assert_eq!(ns, recovered);
    }

    #[test]
    fn namespace_round_trip_forgotten() {
        let ns = MemoryNamespace::Forgotten;
        let (kind, detail) = namespace_to_parts(&ns);
        let recovered = parts_to_namespace(kind, &detail).unwrap();
        assert_eq!(ns, recovered);
    }

    #[test]
    fn value_round_trip_json() {
        let val = MemoryValue::Json(serde_json::json!({"key": "value"}));
        let (kind, data) = value_to_parts(&val).unwrap();
        let recovered = parts_to_value(kind, &data).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn value_round_trip_text() {
        let val = MemoryValue::Text("hello world".into());
        let (kind, data) = value_to_parts(&val).unwrap();
        let recovered = parts_to_value(kind, &data).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn value_round_trip_blob() {
        let val = MemoryValue::Blob(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        let (kind, data) = value_to_parts(&val).unwrap();
        let recovered = parts_to_value(kind, &data).unwrap();
        assert_eq!(val, recovered);
    }

    #[test]
    fn schema_sql_contains_vector_extension() {
        let sql = create_schema_sql(1536);
        assert!(sql.contains("CREATE EXTENSION IF NOT EXISTS vector"));
        assert!(sql.contains("vector(1536)"));
        assert!(sql.contains("hnsw"));
    }
}
