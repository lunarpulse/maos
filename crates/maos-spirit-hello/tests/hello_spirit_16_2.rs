#![forbid(unsafe_code)]

//! Story 16-2 / Trap 11 — the scheduler-loadability bound of `HelloSpirit`
//! (D-16-2-A), placed in `tests/` because inline `#[cfg(test)]` code is
//! charged in the crate's kloc budget.

use maos_spirit_hello::HelloSpirit;

#[test]
fn hello_spirit_is_a_scheduler_loadable_spirit() {
    // D-16-2-A: the scheduler's `load` is generic over
    // `T: Spirit + Send + Sync + 'static` — this bound is the compile-time
    // proof that `HelloSpirit` can take the same path as every other
    // loaded Spirit, and the value proof that the type is unit-constructible
    // exactly as the shell block constructs it.
    fn assert_loadable<T: maos_spirit_abi::lifecycle::Spirit + Send + Sync + 'static>(_: &T) {}
    assert_loadable(&HelloSpirit);
}
