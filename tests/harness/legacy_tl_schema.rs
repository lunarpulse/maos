#![allow(dead_code)]

//! Story 17-8 AC6.2 — the Transparency Log DDL as it was at `1241ce52`, before
//! the participant/consent side tables. Tests that need a "legacy artifact" build
//! it from this verbatim copy — never by dropping tables from a current file —
//! so the fixture is an independently constructed pre-17-8 database.

pub const LEGACY_SCHEMA_SQL: &str = "\
CREATE TABLE IF NOT EXISTS transparency_log (
    frame_id            BLOB    NOT NULL PRIMARY KEY,
    timestamp_ns        INTEGER NOT NULL,
    spirit_pid          INTEGER NOT NULL,
    from_spirit_id      TEXT    NOT NULL DEFAULT '',
    to_spirit_id        TEXT    NOT NULL DEFAULT '',
    boot_nonce          INTEGER NOT NULL,
    capability_token    BLOB,
    kind                INTEGER NOT NULL,
    intent              TEXT    NOT NULL,
    correlation_id      TEXT,
    payload_redacted    BLOB    NOT NULL,
    origin              INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tlog_spirit_pid
    ON transparency_log(spirit_pid, timestamp_ns);
CREATE INDEX IF NOT EXISTS idx_tlog_kind
    ON transparency_log(kind, timestamp_ns);
CREATE INDEX IF NOT EXISTS idx_tlog_boot_pid_ts
    ON transparency_log(boot_nonce, spirit_pid, timestamp_ns, frame_id);

-- Story 6.1 — retraction companion table.
-- Per AC2: original row is APPEND-ONLY-PRESERVED.
-- The retraction marker lives in this companion table, not as a column
-- on transparency_log, to minimize schema migration blast radius.
CREATE TABLE IF NOT EXISTS transparency_log_retractions (
    original_frame_id   BLOB    NOT NULL PRIMARY KEY,
    retract_frame_id    BLOB    NOT NULL,
    retracted_at_ns     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS approval_decision_log (
    decision_id         INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp_ns        INTEGER NOT NULL,
    actor               TEXT    NOT NULL,
    target              TEXT    NOT NULL,
    capability          TEXT    NOT NULL,
    intent              TEXT    NOT NULL,
    decision            INTEGER NOT NULL,
    reasoning           TEXT
);


CREATE INDEX IF NOT EXISTS idx_approval_actor
    ON approval_decision_log(actor, timestamp_ns);

-- Story 9.3b (R10) — schema-lifecycle registry for governance audit.
CREATE TABLE IF NOT EXISTS schema_lifecycle_registry (
    schema_id           TEXT    NOT NULL,
    version             INTEGER NOT NULL,
    effective_at_ns     INTEGER NOT NULL,
    supersedes_hash     TEXT,
    ratified_by         TEXT    NOT NULL,
    recorded_at_ns      INTEGER NOT NULL,
    schema_content_hash TEXT    NOT NULL,
    PRIMARY KEY (schema_id, version)
);

CREATE INDEX IF NOT EXISTS idx_slr_schema_id
    ON schema_lifecycle_registry(schema_id);
CREATE INDEX IF NOT EXISTS idx_slr_version
    ON schema_lifecycle_registry(version);
CREATE INDEX IF NOT EXISTS idx_slr_recorded_at
    ON schema_lifecycle_registry(recorded_at_ns);
";
