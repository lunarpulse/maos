//! Fixture: generic bounds in one order — same inline syntax.
//! cargo-public-api should produce identical output since the bound
//! order is normalized by rustdoc.

pub fn f<T: 'static + Eq>(x: T) -> bool { true }
