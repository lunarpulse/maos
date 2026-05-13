//! Fixture: semantically identical API with different whitespace formatting.
//! The bespoke walker would produce different quote!() strings. cargo-public-api
//! canonical output should be identical regardless of source whitespace.

pub struct Foo { pub x : i32 , }
