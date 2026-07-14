#![forbid(unsafe_code)]

//! Cross-region replication canonical serde for collective-memory KV rows
//! (Story 11.2a).

pub mod bundle;
pub mod leaf;
pub mod router;
