//! trybuild-based compile-error tests for the `#[spirit]` proc-macro.
//!
//! Each `.rs` file under `tests/ui/` should FAIL to compile with a
//! specific error message.

#[test]
#[ignore = "expensive: trybuild compiles every tests/ui case"]
fn spirit_macro_compile_errors() {
    // expensive: trybuild compiles every tests/ui case
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
