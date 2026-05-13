//! Fixture: function moved to inner module WITHOUT reexport.
//! cargo-public-api should detect the removal of the public bar().

mod inner { pub fn bar() -> i32 { 42 } }
