//! Fixture: generic bounds reordered — same inline syntax.
//! cargo-public-api should produce identical output since the bound
//! order is normalized by rustdoc.

pub fn f<T: Eq + 'static>(x: T) -> bool { true }
