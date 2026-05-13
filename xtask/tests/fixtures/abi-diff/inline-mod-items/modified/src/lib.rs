//! Fixture: same inline module, slightly different formatting.
//! cargo-public-api should report zero changes.

pub mod foo { pub fn bar() -> i32 { 1 } }
