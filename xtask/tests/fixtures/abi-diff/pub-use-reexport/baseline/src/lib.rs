//! Fixture: public function defined directly in lib.rs.
//! Modified version moves it behind a pub use reexport.

pub fn bar() -> i32 { 42 }
