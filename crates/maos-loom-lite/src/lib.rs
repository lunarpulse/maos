#![forbid(unsafe_code)]

//! `maos-loom-lite` — Postgres+pgvector collective memory tier (Story 10.4a).
//!
//! This crate implements the Loom-lite collective tier as a user-space service
//! per ADR-006.  The kernel mediates access; Loom-lite stores/indexes content.
//!
//! # Architecture
//!
//! - **Backing store:** Postgres + pgvector (HNSW index, `hnsw.iterative_scan='relaxed_order'`).
//! - **Transport:** MCP-Streamable-HTTP (reuses `maos-mcp` transport, no new transport).
//! - **Kernel bridge:** `CollectiveMemoryPort` adapter crosses the async boundary via
//!   `spawn_blocking` + an injected `tokio::runtime::Handle`.  The kernel stays sync.
//! - **Merkle reuse:** All Merkle operations delegate to `maos-audit` primitives.
//!
//! # Module layout
//!
//! - `store` — Postgres+pgvector backing store (schema, CRUD, vector ops).
//! - `adapter` — `CollectiveMemoryPort` impl with the spawn_blocking bridge.
//! - `migration` — SQLite→Postgres migration engine (AC2).
//! - `schema` — SQL DDL and schema management.

pub mod adapter;
pub mod canonical;
pub mod migration;
pub mod schema;
pub mod replication;
pub mod store;
