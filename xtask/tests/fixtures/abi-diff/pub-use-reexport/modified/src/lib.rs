//! Fixture: function moved to inner module but re-exported.
//! cargo-public-api should report zero changes (reexport preserved).

mod inner { pub fn bar() -> i32 { 42 } }
pub use inner::bar;
