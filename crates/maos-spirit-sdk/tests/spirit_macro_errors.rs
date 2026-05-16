//! trybuild-based compile-error tests for the `#[spirit]` proc-macro.
//!
//! Each `.rs` file under `tests/ui/` should FAIL to compile with a
//! specific error message.

#[test]
fn spirit_macro_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
