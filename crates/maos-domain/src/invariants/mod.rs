//! Invariants I1–I14 codified as Rust types.
//!
//! Each submodule corresponds to one invariant from Architecture §3.2.
//! Every invariant has a marker type (`InvariantI<N>`) and at least one
//! substantive type that expresses the invariant at the type level.

pub mod i1;
pub mod i10;
pub mod i11;
pub mod i12;
pub mod i13;
pub mod i14;
pub mod i2;
pub mod i3;
pub mod i4;
pub mod i5;
pub mod i6;
pub mod i7;
pub mod i8;
pub mod i9;

/// An observation an invariant predicate refused: the invariant and the rule it broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{invariant}: {rule}")]
pub struct Violation {
    pub invariant: &'static str,
    pub rule: &'static str,
}

/// `Ok` when `ok`, else the `Violation` for `invariant`/`rule`.
pub(crate) fn ensure(
    ok: bool,
    invariant: &'static str,
    rule: &'static str,
) -> Result<(), Violation> {
    ok.then_some(()).ok_or(Violation { invariant, rule })
}
