//! Fixture: inline module with public items.
//! Both forms should produce the same cargo-public-api output.

pub mod foo {
    pub fn bar() -> i32 { 1 }
}
